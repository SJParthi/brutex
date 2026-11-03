//! The price band of one stock-day on the F&O strategy path (FNO-BAND).
//!
//! F&O stocks have no fixed circuit. Their operating range is 10% of the base
//! price, the previous close, and it flexes in the move's direction after a
//! cooling-off period (SPEC 10.2, `state/fib-strategy-20261030/SPEC.md`, which
//! cites `sources-leverage-execution.md` lines 21 and 189, T06-03). From
//! 2024-02-05 NSE's daily security file holds the band itself (field `PricRg`,
//! T06-10), and FNO-BAND-FILES (F3) reads it to check this one. This module
//! computes the band for every date from the base price and the flex rule
//! (SPEC OP-12), and every band it computes carries [`LABEL`], "band computed".
//! FNO-LAW records T06-03 in `docs/00-charter.md`; until that row lands the
//! rule is UNVERIFIED in this repository, and both ways the kit reads its flex
//! are computed (below).
//!
//! The band feeds rule 4 of the fill walk (SPEC 9.3): a buy fills nothing in a
//! second whose printed high is at or above the upper limit, and a sell nothing
//! in a second whose printed low is at or below the lower limit, until a second
//! prints off the band. FNO-SEC-EXEC applies that rule. This module hands plain
//! [`Limits`] to FNO-RUN, which adapts them to FNO-SEC-EXEC's `Bands` trait. It
//! does no I/O and emits no telemetry (gate 17): FNO-RUN logs and records.
//!
//! # Two base readings, both computed (PARTS FNO-BAND, "Readings")
//!
//! * **R-BAND-1**, SPEC 10.2's: `C_z(D-1) x rho(D)`, the adjusted Zerodha close
//!   of D-1 times the session's raw/adjusted ratio at its first traded minute,
//!   `rho(D) = G / Z` (SPEC 6.6, R1): G the last GDFL trade in that minute, Z
//!   that Zerodha candle's close. Both prices are of one session, so no
//!   corporate action lies between them and the base holds across a split
//!   ([`Base::close_times_ratio`]).
//! * **R-BAND-2**: GDFL's last trade of D-1, raw ([`Base::last_trade_before`]).
//!
//! Per trade the headline takes the reading that gives the lower net, walked as
//! lanes by FNO-RUN, and only a trade that prints within one price step of a
//! band needs more than one lane ([`DayBand::near`]). D-8210.
//!
//! # Two flex readings, both computed (D-8211)
//!
//! SPEC 10.2 reads T06-03's flex as 5% of the base a step; the FNO-BAND design
//! reads it as 5%, then 3%, then 2%. The source's own words are on the
//! operator's Mac, not in this repository, so which one NSE applies is
//! UNVERIFIED here. [`FlexReading`] holds both, FNO-RUN walks both as lanes and
//! takes the worse per trade (SPEC FIB-I5), and a reading the data refutes (the
//! design's, past its third flex) is refused by name from the second that
//! refutes it ([`Refusal::PastTheLastFlex`]).
//!
//! # Observed, never predicted (D-8210)
//!
//! The data cannot show NSE's flex; it shows prints. No trade prints outside
//! the band in force, so a print past a limit says the band had flexed by that
//! second, at least as far as the print. [`DayBand::observe`] widens that side
//! by the fewest steps of its reading that hold the print, and the flex is in
//! force from that second. A print exactly at a limit is at the band, not past
//! it. The two sides flex apart, each in its move's direction, and the band
//! never narrows within its day. Only traded prints are observed: the quote
//! range of the operator's N4 decision is never a trade, so it widens nothing.
//!
//! # No look-ahead (SPEC FIB-I1)
//!
//! [`DayBand::limits`] answers the band in force at a second from the prints
//! observed at or before it. Each flex is kept with its second, so a print
//! observed later never moves an earlier answer, whatever order the caller asks
//! in. `runner::fno_band::tests::no_later_second_moves_an_earlier_band` proves it
//! on seeded random sessions.
//!
//! # Integers only (SPEC FIB-I10)
//!
//! Prices are paise `i64`, shares of the base are parts per million, and every
//! product is taken in `i128`. Limits round inward to the paisa: the upper one
//! down and the lower one up, the narrowest band the rule allows (OP-12, "the
//! strict way"). A band that cannot be held in those types is refused by name
//! ([`Refusal::OutOfRange`], [`Refusal::BaseOutOfRange`]), never wrapped or
//! saturated. D-8212.
//!
//! # Cost
//!
//! [`DayBand::observe`] is a fixed number of integer operations and at most one
//! push per side that flexes. [`DayBand::limits`] at or after a side's latest
//! flex reads that flex; at an earlier second it binary-searches that side's
//! flexes of the day, which are few: three at most under the design's reading.
//! C-FNO-BAND-01 times one check (one `observe` and one `limits` at the walk's
//! second) and one look-back at 1x, 10x and 100x the seconds of a day in
//! `crates/runner/benches/ratio.rs`, through [`measure_checks`], which
//! `runner::fno_band::tests::the_bench_fixture_times_every_second_it_walks` runs
//! at a tiny size; the row fails unless both are flat within the bench's
//! ceiling and inside their budget.

use core::fmt;
use std::hint::black_box;
use std::time::Instant;

/// The label every band this module computes carries, word for word (PARTS
/// FNO-BAND: labelled "band computed"; SPEC OP-12).
pub const LABEL: &str = "band computed";

/// The band rule's word for the run identity's `band_rule` term (PARTS,
/// READINESS ADDENDUM item 2: FNO-SEC-WIRE builds `FnoRunV1`). It names this
/// module's rule whole: the operating range, both base readings and both flex
/// readings, every one walked. A change to the rule is a new word, never an
/// edit of this one, so a run under another rule is another run. D-8210.
pub const BAND_RULE: &str = "fno-band-v1";

/// The operating range on each side of the base before any flex: 10% of the
/// base, in parts per million (SPEC 10.2; T06-03).
pub const RANGE_PPM: u64 = 100_000;

/// One flex under SPEC 10.2's reading of T06-03: 5% of the base, in parts per
/// million, with no last flex.
pub const SPEC_FLEX_PPM: u64 = 50_000;

/// Each side's half-width after its first, second and third flex under the
/// FNO-BAND design's reading of T06-03, "5%, then 3%, then 2%": 15%, 18% and
/// 20% of the base, in parts per million. There is no fourth.
pub const DESIGN_WIDTHS_PPM: [u64; 3] = [150_000, 180_000, 200_000];

/// Parts per million in one whole.
const PPM: i128 = 1_000_000;

/// How a side of the band flexes: the two readings of T06-03 the kit holds
/// (D-8211).
///
/// SPEC 10.2 reads a flex as 5% of the base a step; the FNO-BAND design (PARTS
/// section 5) reads it as 5%, then 3%, then 2%. The source's own words are in
/// `sources-leverage-execution.md` lines 21 and 189, which this repository
/// does not hold, so which one NSE applies is UNVERIFIED here. Both are
/// computed, FNO-RUN walks both as lanes, and the headline takes the worse per
/// trade (SPEC FIB-I5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlexReading {
    /// SPEC 10.2: each flex widens its side by [`SPEC_FLEX_PPM`], with no last
    /// flex.
    FivePerStep,
    /// The FNO-BAND design: a side's half-width steps through
    /// [`DESIGN_WIDTHS_PPM`], and a print past the last is a reading the data
    /// refutes ([`Refusal::PastTheLastFlex`]).
    FiveThreeTwo,
}

impl FlexReading {
    /// Both readings, in a fixed order.
    pub const ALL: [Self; 2] = [Self::FivePerStep, Self::FiveThreeTwo];

    /// The reading's name in records and labels.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::FivePerStep => "flex 5% a step (SPEC 10.2)",
            Self::FiveThreeTwo => "flex 5%, then 3%, then 2% (FNO-BAND design)",
        }
    }

    /// The narrowest half-width this reading reaches that is at least
    /// `required` parts per million, or `None` when it reaches none.
    fn width_holding(self, required: i128) -> Option<i128> {
        match self {
            Self::FivePerStep => {
                let range = i128::from(RANGE_PPM);
                let step = i128::from(SPEC_FLEX_PPM);
                let over = (required - range).max(0);
                Some(range + (over + step - 1) / step * step)
            }
            Self::FiveThreeTwo => DESIGN_WIDTHS_PPM
                .into_iter()
                .map(i128::from)
                .find(|&width| width >= required),
        }
    }
}

/// Which reading of the base price a band stands on (PARTS FNO-BAND,
/// "Readings").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseReading {
    /// R-BAND-1, SPEC 10.2's: the adjusted Zerodha close of D-1 times the
    /// session ratio `rho(D)`.
    CloseTimesRatio,
    /// R-BAND-2: GDFL's last trade of D-1, raw.
    LastTradeBefore,
}

impl BaseReading {
    /// Both readings, in a fixed order.
    pub const ALL: [Self; 2] = [Self::CloseTimesRatio, Self::LastTradeBefore];

    /// The reading's id in records and labels.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::CloseTimesRatio => "R-BAND-1",
            Self::LastTradeBefore => "R-BAND-2",
        }
    }
}

/// What the day mask knows of a corporate action going ex on the band's day
/// (SPEC 3.2: FNO-DAYMASK's M7 and M11 skip demerger ex-days).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExDay {
    /// No demerger goes ex on the day, as far as the day mask knows.
    Ordinary,
    /// A demerger goes ex on the day. Prices after a demerger's ex-date are on
    /// a new basis, and no table adjusts either reading's base across one
    /// until the corporate-action table (R4-26) is live, so the band refuses
    /// the day by name ([`Refusal::DemergerExDay`]), as the day mask skips it.
    Demerger,
}

/// The session's raw/adjusted ratio `rho(D)` at its first traded minute (SPEC
/// 6.6, R1; SPEC 10.2): `raw / adjusted`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SessionRatio {
    /// G: the last GDFL trade inside the session's first traded minute, raw
    /// paise.
    pub raw: i64,
    /// Z: the close of that minute's Zerodha 1-minute candle, adjusted paise.
    pub adjusted: i64,
}

/// A base price in raw paise, the reading it came from, and its band before any
/// flex. Only the two readings' constructors make one, so a band never stands
/// on a base the rule does not name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Base {
    reading: BaseReading,
    paise: i64,
    first: Limits,
}

impl Base {
    /// R-BAND-1: `C_z(D-1) x G / Z`, snapped to the paisa half-up, once
    /// (CLAUDE.md section 7). `prev_close` is `C_z(D-1)`, adjusted paise.
    ///
    /// # Errors
    ///
    /// [`Refusal::DemergerExDay`] on a demerger ex-day;
    /// [`Refusal::NotPositive`] when a price is not a positive number of paise
    /// or the base snaps to none; [`Refusal::BaseOutOfRange`] when the base or
    /// its band before any flex does not fit paise `i64`.
    pub fn close_times_ratio(
        day: ExDay,
        prev_close: i64,
        ratio: SessionRatio,
    ) -> Result<Self, Refusal> {
        refuse_demerger(day)?;
        let prev = i128::from(positive("the adjusted close of D-1", prev_close)?);
        let raw = i128::from(positive("the session ratio's raw price", ratio.raw)?);
        let adjusted = i128::from(positive(
            "the session ratio's adjusted price",
            ratio.adjusted,
        )?);
        // Each factor is at most i64::MAX, so 2 x prev x raw + adjusted is at
        // most 2 (2^63 - 1)^2 + 2^63 - 1, which is below i128::MAX by more than
        // 2^64: the half-up numerator cannot overflow.
        let snapped = (2 * prev * raw + adjusted) / (2 * adjusted);
        Self::of(BaseReading::CloseTimesRatio, snapped)
    }

    /// R-BAND-2: GDFL's last trade of D-1, raw paise, as given.
    ///
    /// # Errors
    ///
    /// [`Refusal::DemergerExDay`] on a demerger ex-day;
    /// [`Refusal::NotPositive`] when the trade is not a positive number of
    /// paise; [`Refusal::BaseOutOfRange`] when its band before any flex does
    /// not fit paise `i64`.
    pub fn last_trade_before(day: ExDay, last_trade: i64) -> Result<Self, Refusal> {
        refuse_demerger(day)?;
        let paise = positive("the last trade of D-1", last_trade)?;
        Self::of(BaseReading::LastTradeBefore, i128::from(paise))
    }

    /// The base of `reading` at `paise`, with its band before any flex: the
    /// base and 10% of it either way, rounded inward.
    fn of(reading: BaseReading, paise: i128) -> Result<Self, Refusal> {
        let Ok(base) = i64::try_from(paise) else {
            return Err(Refusal::BaseOutOfRange { reading });
        };
        if base < 1 {
            return Err(Refusal::NotPositive {
                what: "the base price",
                value: base,
            });
        }
        let Ok(upper) = i64::try_from(paise + paise * i128::from(RANGE_PPM) / PPM) else {
            return Err(Refusal::BaseOutOfRange { reading });
        };
        // upper - base is a tenth of the base, rounded down, so base minus it
        // is at least 0.9 x base: plain i64 arithmetic, in range.
        let lower = base - (upper - base);
        Ok(Self {
            reading,
            paise: base,
            first: Limits { lower, upper },
        })
    }

    /// The reading this base came from.
    #[must_use]
    pub const fn reading(self) -> BaseReading {
        self.reading
    }

    /// The base price, raw paise.
    #[must_use]
    pub const fn paise(self) -> i64 {
        self.paise
    }

    /// The band before any flex: the base and 10% of it either way, rounded
    /// inward to the paisa.
    #[must_use]
    pub const fn first_limits(self) -> Limits {
        self.first
    }
}

/// `value`, if it is a positive number of paise.
fn positive(what: &'static str, value: i64) -> Result<i64, Refusal> {
    if value < 1 {
        Err(Refusal::NotPositive { what, value })
    } else {
        Ok(value)
    }
}

/// Refuses a demerger ex-day ([`ExDay::Demerger`]).
const fn refuse_demerger(day: ExDay) -> Result<(), Refusal> {
    match day {
        ExDay::Ordinary => Ok(()),
        ExDay::Demerger => Err(Refusal::DemergerExDay),
    }
}

/// One traded second's printed range, raw paise: what the band observes
/// (PARTS FNO-BAND, `observe(sec)`). Only trades: the quote range of the N4
/// decision is never a trade, so it widens nothing (D-8210).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prints {
    /// The second's stamp. The band orders and compares stamps and reads
    /// nothing else in them.
    pub ts: i64,
    /// The highest price traded in the second.
    pub high: i64,
    /// The lowest price traded in the second.
    pub low: i64,
}

/// A band's two limits, raw paise. A buy fills nothing in a second whose high
/// is at or above `upper`, and a sell nothing in one whose low is at or below
/// `lower` (SPEC 9.3, rule 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Limits {
    /// The lower limit.
    pub lower: i64,
    /// The upper limit.
    pub upper: i64,
}

impl Limits {
    /// The plain pair `(lower, upper)` FNO-RUN hands to FNO-SEC-EXEC's `Bands`
    /// trait (PARTS FNO-BAND, "Depends on").
    #[must_use]
    pub const fn pair(self) -> (i64, i64) {
        (self.lower, self.upper)
    }
}

/// A side of the band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BandSide {
    /// The upper limit, which a rise flexes.
    Upper,
    /// The lower limit, which a fall flexes.
    Lower,
}

/// One side's flex in one second: its half-width before and after, in parts
/// per million of the base.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Flex {
    /// The half-width in force before the second.
    pub from_ppm: u64,
    /// The half-width in force from the second on.
    pub to_ppm: u64,
}

/// What one observed second did, for the caller's records (SPEC FIB-I12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Observed {
    /// The band in force in the second, after any flex the second shows.
    pub limits: Limits,
    /// The upper side's flex in the second, if it flexed.
    pub up: Option<Flex>,
    /// The lower side's flex in the second, if it flexed.
    pub down: Option<Flex>,
    /// Whether the second's high is at the upper limit in force: a buy fills
    /// nothing in it (rule 4).
    pub at_upper: bool,
    /// Whether the second's low is at the lower limit in force: a sell fills
    /// nothing in it (rule 4).
    pub at_lower: bool,
}

/// Why the band refused, by name (CLAUDE.md section 4: no fallback that hides
/// a failure). D-8212.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// A demerger goes ex on the band's day ([`ExDay::Demerger`]).
    DemergerExDay,
    /// A price the base needs is not a positive number of paise, or the base
    /// snapped to none.
    NotPositive {
        /// Which price.
        what: &'static str,
        /// The value given, or the snapped base.
        value: i64,
    },
    /// The base, or its band before any flex, does not fit paise `i64`.
    BaseOutOfRange {
        /// The reading whose base it was.
        reading: BaseReading,
    },
    /// A second whose low is under one paisa or above its high.
    BadPrints {
        /// The second's stamp.
        ts: i64,
        /// Its high.
        high: i64,
        /// Its low.
        low: i64,
    },
    /// A second earlier than one the band already observed: a band is built
    /// in time order.
    OutOfOrder {
        /// The second offered.
        ts: i64,
        /// The latest second already observed.
        last: i64,
    },
    /// A negative price step asked of [`DayBand::near`].
    NegativeStep {
        /// The step given.
        step: i64,
    },
    /// The band's flex reading has no flex wide enough to hold a print: the
    /// data refutes the reading for this stock-day from this second on.
    PastTheLastFlex {
        /// The second that refutes it.
        ts: i64,
        /// The side the print is past.
        side: BandSide,
        /// The print.
        price: i64,
    },
    /// The flex a print needs cannot be held in the band's integer types: a
    /// half-width in `u64` parts per million, a limit in paise `i64`.
    OutOfRange {
        /// The second whose print needs it.
        ts: i64,
        /// The side the print is past.
        side: BandSide,
        /// The print.
        price: i64,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::DemergerExDay => write!(
                f,
                "no band: a demerger goes ex on this day, so neither base reading holds"
            ),
            Self::NotPositive { what, value } => {
                write!(f, "no band: {what} is {value} paise, not a positive price")
            }
            Self::BaseOutOfRange { reading } => write!(
                f,
                "no band: the {} base or its first band does not fit paise i64",
                reading.id()
            ),
            Self::BadPrints { ts, high, low } => write!(
                f,
                "second {ts} refused: low {low} and high {high} are not a traded range"
            ),
            Self::OutOfOrder { ts, last } => {
                write!(f, "second {ts} refused: second {last} was already observed")
            }
            Self::NegativeStep { step } => {
                write!(f, "a price step of {step} paise is negative")
            }
            Self::PastTheLastFlex { ts, side, price } => write!(
                f,
                "band refuted at second {ts}: {price} is past the {side:?} limit's last flex"
            ),
            Self::OutOfRange { ts, side, price } => write!(
                f,
                "band refused at second {ts}: the {side:?} flex {price} needs does not fit the band's integers"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// A side's half-width and limit from a second on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mark {
    /// The first second it is in force; `i64::MIN` for the band before any
    /// flex, which is in force from before the first second.
    at: i64,
    /// The half-width, parts per million of the base.
    width: u64,
    /// The limit, raw paise.
    limit: i64,
}

/// One side of a band: its mark before any flex and every flex since, in time
/// order.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Edge {
    first: Mark,
    flexes: Vec<Mark>,
}

impl Edge {
    /// A side with no flex, at `limit`.
    const fn new(limit: i64) -> Self {
        Self {
            first: Mark {
                at: i64::MIN,
                width: RANGE_PPM,
                limit,
            },
            flexes: Vec::new(),
        }
    }

    /// The mark in force after every second observed.
    fn now(&self) -> Mark {
        self.flexes.last().copied().unwrap_or(self.first)
    }

    /// The mark in force at second `ts`: the latest flex at or before it.
    fn at(&self, ts: i64) -> Mark {
        let flexed = self.flexes.partition_point(|mark| mark.at <= ts);
        flexed
            .checked_sub(1)
            .and_then(|latest| self.flexes.get(latest))
            .copied()
            .unwrap_or(self.first)
    }

    /// Records `mark` as this side's latest flex, and says what it changed.
    fn push(&mut self, mark: Mark) -> Flex {
        let from_ppm = self.now().width;
        self.flexes.push(mark);
        Flex {
            from_ppm,
            to_ppm: mark.width,
        }
    }
}

/// The price band of one stock-day under one base reading and one flex
/// reading (PARTS FNO-BAND).
///
/// Built with [`Self::new`], fed every traded second of the day in time order
/// through [`Self::observe`], and asked [`Self::limits`] at any second.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayBand {
    base: Base,
    flex: FlexReading,
    upper: Edge,
    lower: Edge,
    last: Option<i64>,
    broken: Option<(i64, Refusal)>,
}

impl DayBand {
    /// The band of one stock-day standing on `base` and flexing by `flex`.
    ///
    /// The design names this `DayBand::new(base_reading, base)`: the reading
    /// travels inside [`Base`], so the two can never disagree, and the flex
    /// reading is the second argument because SPEC 10.2 and the design read
    /// T06-03 two ways (D-8211).
    #[must_use]
    pub const fn new(base: Base, flex: FlexReading) -> Self {
        Self {
            base,
            flex,
            upper: Edge::new(base.first.upper),
            lower: Edge::new(base.first.lower),
            last: None,
            broken: None,
        }
    }

    /// The base this band stands on.
    #[must_use]
    pub const fn base(&self) -> Base {
        self.base
    }

    /// The flex reading this band flexes by.
    #[must_use]
    pub const fn flex(&self) -> FlexReading {
        self.flex
    }

    /// Observes one traded second (PARTS FNO-BAND, `observe(sec)`).
    ///
    /// A print past a limit widens that side by the fewest steps of the flex
    /// reading that hold it, in force from this second; a print at a limit is
    /// at the band and widens nothing. A second already observed may be
    /// observed again.
    ///
    /// # Errors
    ///
    /// [`Refusal::BadPrints`] and [`Refusal::OutOfOrder`] change nothing.
    /// [`Refusal::PastTheLastFlex`] or [`Refusal::OutOfRange`] break the band
    /// from this second: every later call returns that same refusal, and the
    /// answers for earlier seconds stand.
    pub fn observe(&mut self, sec: Prints) -> Result<Observed, Refusal> {
        check(sec)?;
        if let Some(last) = self.last
            && sec.ts < last
        {
            return Err(Refusal::OutOfOrder { ts: sec.ts, last });
        }
        if let Some((_, why)) = self.broken {
            return Err(why);
        }
        let marks = self
            .reach(BandSide::Upper, sec.ts, sec.high)
            .and_then(|up| {
                self.reach(BandSide::Lower, sec.ts, sec.low)
                    .map(|down| (up, down))
            });
        self.last = Some(sec.ts);
        let (up, down) = match marks {
            Ok(marks) => marks,
            Err(why) => {
                self.broken = Some((sec.ts, why));
                return Err(why);
            }
        };
        let up = up.map(|mark| self.upper.push(mark));
        let down = down.map(|mark| self.lower.push(mark));
        let limits = Limits {
            lower: self.lower.now().limit,
            upper: self.upper.now().limit,
        };
        Ok(Observed {
            limits,
            up,
            down,
            at_upper: sec.high == limits.upper,
            at_lower: sec.low == limits.lower,
        })
    }

    /// The flex a print at `price` past `side`'s limit needs at second `ts`, or
    /// `None` when the limit in force holds it.
    fn reach(&self, side: BandSide, ts: i64, price: i64) -> Result<Option<Mark>, Refusal> {
        let base = i128::from(self.base.paise);
        let at = i128::from(price);
        let (holds, gap) = match side {
            BandSide::Upper => (price <= self.upper.now().limit, at - base),
            BandSide::Lower => (price >= self.lower.now().limit, base - at),
        };
        if holds {
            return Ok(None);
        }
        // The narrowest half-width whose limit holds the print: the limit is
        // base + floor(base x w / PPM) above and base - floor(base x w / PPM)
        // below, so it holds a print `gap` from the base exactly when
        // base x w >= gap x PPM. `gap` is under 2^64 and `base` at least one,
        // so every product below fits i128.
        let required = (gap * PPM + base - 1) / base;
        let Some(width) = self.flex.width_holding(required) else {
            return Err(Refusal::PastTheLastFlex { ts, side, price });
        };
        let offset = width * base / PPM;
        let limit = match side {
            BandSide::Upper => base + offset,
            BandSide::Lower => base - offset,
        };
        let (Ok(width), Ok(limit)) = (u64::try_from(width), i64::try_from(limit)) else {
            return Err(Refusal::OutOfRange { ts, side, price });
        };
        Ok(Some(Mark {
            at: ts,
            width,
            limit,
        }))
    }

    /// The band in force at second `ts`, from the prints observed at or before
    /// it (PARTS FNO-BAND, `limits(ts)`).
    ///
    /// A print observed after `ts` never moves the answer. A second the caller
    /// has not observed yet is answered with what has been observed, so the
    /// caller observes a second before it asks about it.
    ///
    /// # Errors
    ///
    /// The refusal that broke the band, for a second at or after the one that
    /// broke it.
    pub fn limits(&self, ts: i64) -> Result<Limits, Refusal> {
        if let Some((at, why)) = self.broken
            && ts >= at
        {
            return Err(why);
        }
        Ok(Limits {
            lower: self.lower.at(ts).limit,
            upper: self.upper.at(ts).limit,
        })
    }

    /// Whether `sec` prints within `step` paise of a limit in force at its
    /// second, or past one: the lane test of PARTS FNO-BAND ("only trades that
    /// print within one step of a band need two lanes"). `step` is the stock's
    /// measured price step (SPEC 10.1), the caller's to give; zero asks
    /// whether the second is at or past a limit.
    ///
    /// # Errors
    ///
    /// [`Refusal::NegativeStep`], [`Refusal::BadPrints`], and the refusal that
    /// broke the band, as [`Self::limits`] gives it.
    pub fn near(&self, sec: Prints, step: i64) -> Result<bool, Refusal> {
        if step < 0 {
            return Err(Refusal::NegativeStep { step });
        }
        check(sec)?;
        let limits = self.limits(sec.ts)?;
        Ok(sec.high.saturating_add(step) >= limits.upper
            || sec.low.saturating_sub(step) <= limits.lower)
    }
}

/// Refuses a second whose prints are not a traded range.
const fn check(sec: Prints) -> Result<(), Refusal> {
    if sec.low < 1 || sec.high < sec.low {
        Err(Refusal::BadPrints {
            ts: sec.ts,
            high: sec.high,
            low: sec.low,
        })
    } else {
        Ok(())
    }
}

/// Timed band checks (C-FNO-BAND-01).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Samples {
    /// Nanoseconds of each timed check, in walk order: one
    /// [`DayBand::observe`] and one [`DayBand::limits`] at the same second,
    /// the fill walk's use.
    pub check_ns: Vec<u64>,
    /// Nanoseconds of each timed look-back, in walk order: one
    /// [`DayBand::limits`] at an earlier second of the day.
    pub lookback_ns: Vec<u64>,
    /// Flexes the walk observed, both sides. A walk that never flexed timed no
    /// history.
    pub flexes: u64,
    /// Timed [`DayBand::limits`] calls that refused. A walk with one timed a
    /// refusal, not a band.
    pub refused: u64,
}

/// The nearest-rank `permille`-th of `samples`: the smallest sample with at
/// least that share of them at or below it. `None` for no samples, or a
/// `permille` past 1,000.
#[must_use]
pub fn quantile(samples: &[u64], permille: u16) -> Option<u64> {
    if permille > 1_000 {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = sorted
        .len()
        .saturating_mul(usize::from(permille))
        .div_ceil(1_000);
    sorted.get(rank.saturating_sub(1)).copied()
}

/// The bench's base: R-BAND-2 at Rs 1,000.00, a band of Rs 900.00 to
/// Rs 1,100.00 before any flex.
const BENCH_BASE: Base = Base {
    reading: BaseReading::LastTradeBefore,
    paise: 100_000,
    first: Limits {
        lower: 90_000,
        upper: 110_000,
    },
};

/// The bench's stock-day under `flex`: a band on [`BENCH_BASE`] and `seconds`
/// traded seconds, one a second from stamp 0. The first second prints 12% up
/// and the second 12% down, so each side flexes once, to 15%, under either
/// reading, and every later check meets a band with history; every later
/// second prints between 14% down and 14% up, by a seeded xorshift, and flexes
/// nothing more.
#[must_use]
pub fn bench_day(flex: FlexReading, seconds: u32) -> (DayBand, Vec<Prints>) {
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let day = (0..i64::from(seconds))
        .map(|ts| {
            state = xorshift(state);
            let wander = i64::from(u16::try_from(state >> 48).unwrap_or(0)) % 28_000;
            let (high, low) = match ts {
                0 => (112_000, 112_000),
                1 => (88_000, 88_000),
                _ => (86_000 + wander + wander % 3, 86_000 + wander),
            };
            Prints { ts, high, low }
        })
        .collect();
    (DayBand::new(BENCH_BASE, flex), day)
}

/// Times one check and one look-back for each second of `day` on `band`
/// (C-FNO-BAND-01). The check is one [`DayBand::observe`] and one
/// [`DayBand::limits`] at the same second; the look-back is one
/// [`DayBand::limits`] at a second drawn from the day's first to this one.
///
/// # Errors
///
/// The first refusal [`DayBand::observe`] gives, which the fixture of
/// [`bench_day`] never meets.
pub fn measure_checks(band: &mut DayBand, day: &[Prints]) -> Result<Samples, Refusal> {
    let mut samples = Samples {
        check_ns: Vec::with_capacity(day.len()),
        lookback_ns: Vec::with_capacity(day.len()),
        flexes: 0,
        refused: 0,
    };
    let first = day.first().map_or(0, |sec| sec.ts);
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    for &sec in day {
        let began = Instant::now();
        let observed = black_box(band.observe(black_box(sec)))?;
        let now = black_box(band.limits(black_box(sec.ts)));
        let check_ns = elapsed_ns(began);
        state = xorshift(state);
        // `observe` has taken `sec`, so `sec.ts` is at or after `first` and the
        // span is at least one. The draw is under 2^31 and under the span, so
        // the stamp it picks fits i64 and lies in `first ..= sec.ts`.
        let span = i128::from(sec.ts) - i128::from(first) + 1;
        let drawn = i128::from(state >> 33) % span;
        let back = first + i64::try_from(drawn).unwrap_or(0);
        let began = Instant::now();
        let then = black_box(band.limits(black_box(back)));
        let lookback_ns = elapsed_ns(began);
        samples.check_ns.push(check_ns);
        samples.lookback_ns.push(lookback_ns);
        samples.flexes += u64::from(observed.up.is_some()) + u64::from(observed.down.is_some());
        samples.refused += u64::from(now.is_err()) + u64::from(then.is_err());
    }
    Ok(samples)
}

/// Marsaglia's xorshift64 step: a fixed function of its input on every machine
/// (CLAUDE.md section 3 rule 5).
const fn xorshift(mut state: u64) -> u64 {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    state
}

/// Nanoseconds since `began`, saturated.
fn elapsed_ns(began: Instant) -> u64 {
    u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]
mod tests {
    use super::{
        BAND_RULE, BandSide, Base, BaseReading, DESIGN_WIDTHS_PPM, DayBand, ExDay, Flex,
        FlexReading, LABEL, Limits, Observed, PPM, Prints, RANGE_PPM, Refusal, SPEC_FLEX_PPM,
        SessionRatio, bench_day, measure_checks, quantile, xorshift,
    };

    /// Rs 1,000.00: the base most cases stand on.
    const RS_1000: i64 = 100_000;

    /// R-BAND-2 at `paise` on an ordinary day.
    fn base(paise: i64) -> Base {
        Base::last_trade_before(ExDay::Ordinary, paise).expect("a positive base in range")
    }

    /// A band on R-BAND-2 at `paise`.
    fn band(paise: i64, flex: FlexReading) -> DayBand {
        DayBand::new(base(paise), flex)
    }

    /// A second that printed one price.
    const fn at(ts: i64, price: i64) -> Prints {
        Prints {
            ts,
            high: price,
            low: price,
        }
    }

    /// A second that printed from `low` to `high`.
    const fn range(ts: i64, high: i64, low: i64) -> Prints {
        Prints { ts, high, low }
    }

    const fn limits(lower: i64, upper: i64) -> Limits {
        Limits { lower, upper }
    }

    /// Rule 4 for a buy (SPEC 9.3): it fills nothing in this second.
    const fn buy_blocked(band: Limits, sec: Prints) -> bool {
        sec.high >= band.upper
    }

    /// Rule 4 for a sell (SPEC 9.3): it fills nothing in this second.
    const fn sell_blocked(band: Limits, sec: Prints) -> bool {
        sec.low <= band.lower
    }

    fn observe(band: &mut DayBand, sec: Prints) -> Observed {
        band.observe(sec).expect("the band observes this second")
    }

    fn limits_at(band: &DayBand, ts: i64) -> Limits {
        band.limits(ts).expect("the band answers this second")
    }

    /// A limit at half-width `width` ppm on `side` of `base`, rounded inward.
    fn limit_of(base: i64, width: u64, side: BandSide) -> i64 {
        let offset =
            i64::try_from(i128::from(base) * i128::from(width) / PPM).expect("an offset in range");
        match side {
            BandSide::Upper => base + offset,
            BandSide::Lower => base - offset,
        }
    }

    /// The rung of a reading's ladder just below a flexed half-width `width`.
    fn width_below(flex: FlexReading, width: u64) -> u64 {
        match flex {
            FlexReading::FivePerStep => width - SPEC_FLEX_PPM,
            FlexReading::FiveThreeTwo => [RANGE_PPM]
                .into_iter()
                .chain(DESIGN_WIDTHS_PPM)
                .filter(|&rung| rung < width)
                .max()
                .expect("a flexed width has a rung below it"),
        }
    }

    /// Marsaglia's xorshift64, seeded away from zero.
    struct Rng(u64);

    impl Rng {
        fn seeded(seed: u64) -> Self {
            Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
        }

        fn below(&mut self, n: u64) -> i64 {
            self.0 = xorshift(self.0);
            i64::try_from(self.0 % n).expect("under n")
        }
    }

    /// A random session of `len` traded seconds around `base`: stamps from
    /// 09:15:00 with gaps of up to 90 seconds, small moves, now and then a
    /// jump of 5% to 34% either way, so bands flex and the design's reading is
    /// sometimes refuted, and now and then a lock exactly at the first band,
    /// so seconds sit at a limit.
    fn session(rng: &mut Rng, base: i64, len: usize) -> Vec<Prints> {
        let mut ts = 33_300;
        let mut price = base;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            ts += 1 + rng.below(90);
            let up = rng.below(2) == 0;
            price = match rng.below(40) {
                0 => {
                    let size = base * (5 + rng.below(30)) / 100;
                    if up { price + size } else { price - size }
                }
                1 => {
                    let first = base / 10;
                    if up { base + first } else { base - first }
                }
                _ => {
                    let size = base / 400 + 1;
                    if up { price + size } else { price - size }
                }
            }
            .max(1);
            let spread = rng.below(3) * (base / 1_000 + 1);
            out.push(range(ts, price + spread, price));
        }
        out
    }

    /// The design's first named case: prints exactly at the band.
    #[test]
    fn prints_exactly_at_the_band_are_at_it_and_flex_nothing() {
        for flex in FlexReading::ALL {
            let mut day = band(RS_1000, flex);
            assert_eq!(day.base().first_limits(), limits(90_000, 110_000));
            assert_eq!(
                observe(&mut day, at(0, 110_000)),
                Observed {
                    limits: limits(90_000, 110_000),
                    up: None,
                    down: None,
                    at_upper: true,
                    at_lower: false,
                },
                "{flex:?}: a print at the upper limit is at the band, not past it"
            );
            assert_eq!(
                observe(&mut day, at(1, 90_000)),
                Observed {
                    limits: limits(90_000, 110_000),
                    up: None,
                    down: None,
                    at_upper: false,
                    at_lower: true,
                },
                "{flex:?}: a print at the lower limit is at the band, not past it"
            );
            let both = observe(&mut day, range(2, 110_000, 90_000));
            assert_eq!(
                (both.at_upper, both.at_lower, both.up, both.down),
                (true, true, None, None)
            );
            assert_eq!(limits_at(&day, 2), limits(90_000, 110_000));
            // One paisa past is a flex: the same price one step on.
            let past = observe(&mut day, at(3, 110_001));
            assert_eq!(
                past.up,
                Some(Flex {
                    from_ppm: 100_000,
                    to_ppm: 150_000
                })
            );
            assert_eq!(past.limits, limits(90_000, 115_000));
            assert!(!past.at_upper);
        }
    }

    /// The design's second named case: a flex observed, under both readings,
    /// and every flex of each.
    #[test]
    fn a_flex_is_observed_from_a_print_past_the_band_and_holds_from_its_second() {
        let flexes = |flex| {
            let mut day = band(RS_1000, flex);
            let trail: Vec<Result<Observed, Refusal>> = [
                at(10, 112_000),
                at(20, 116_000),
                at(30, 119_000),
                at(40, 121_000),
            ]
            .into_iter()
            .map(|sec| day.observe(sec))
            .collect();
            (day, trail)
        };
        let (spec, trail) = flexes(FlexReading::FivePerStep);
        let uppers: Vec<Option<(i64, Option<Flex>)>> = trail
            .iter()
            .map(|seen| seen.ok().map(|seen| (seen.limits.upper, seen.up)))
            .collect();
        let flex = |from_ppm, to_ppm| Some(Flex { from_ppm, to_ppm });
        assert_eq!(
            uppers,
            [
                Some((115_000, flex(100_000, 150_000))),
                Some((120_000, flex(150_000, 200_000))),
                Some((120_000, None)),
                Some((125_000, flex(200_000, 250_000))),
            ],
            "SPEC 10.2's reading: 15%, 20%, 20% held, 25%"
        );
        // Each flex is in force from its own second, and not before it.
        assert_eq!(limits_at(&spec, 9).upper, 110_000);
        assert_eq!(limits_at(&spec, 10).upper, 115_000);
        assert_eq!(limits_at(&spec, 19).upper, 115_000);
        assert_eq!(limits_at(&spec, 20).upper, 120_000);
        assert_eq!(limits_at(&spec, 40).upper, 125_000);
        assert_eq!(limits_at(&spec, 40).lower, 90_000, "a rise flexes no fall");

        let (design, trail) = flexes(FlexReading::FiveThreeTwo);
        assert_eq!(
            trail[..3]
                .iter()
                .map(|seen| seen.ok().map(|seen| (seen.limits.upper, seen.up)))
                .collect::<Vec<_>>(),
            [
                Some((115_000, flex(100_000, 150_000))),
                Some((118_000, flex(150_000, 180_000))),
                Some((120_000, flex(180_000, 200_000))),
            ],
            "the design's reading: 15%, 18%, 20%"
        );
        assert_eq!(
            trail[3],
            Err(Refusal::PastTheLastFlex {
                ts: 40,
                side: BandSide::Upper,
                price: 121_000
            }),
            "21% is past the design's last flex: the reading is refuted by name"
        );
        assert_eq!(limits_at(&design, 39).upper, 120_000);

        // A fall flexes the lower side alone, under either reading.
        for flex in FlexReading::ALL {
            let mut day = band(RS_1000, flex);
            let seen = observe(&mut day, at(50, 87_000));
            assert_eq!(
                seen.down,
                Some(Flex {
                    from_ppm: 100_000,
                    to_ppm: 150_000
                })
            );
            assert_eq!(seen.up, None);
            assert_eq!(seen.limits, limits(85_000, 110_000));
        }
    }

    /// The design's third named case: a sell at the lower band and a buy at the
    /// upper band, read through rule 4 against the limits handed on.
    #[test]
    fn a_sell_at_the_lower_band_and_a_buy_at_the_upper_band() {
        for flex in FlexReading::ALL {
            let mut day = band(50_000, flex);
            let top = at(100, 55_000);
            let seen = observe(&mut day, top);
            let (lower, upper) = limits_at(&day, 100).pair();
            assert_eq!((lower, upper), (45_000, 55_000));
            assert_eq!(
                (seen.at_upper, buy_blocked(seen.limits, top)),
                (true, true),
                "a buy at the upper band fills nothing"
            );
            assert!(
                !sell_blocked(seen.limits, top),
                "a sell at the upper band is not held by it"
            );

            let bottom = at(101, 45_000);
            let seen = observe(&mut day, bottom);
            assert_eq!(
                (seen.at_lower, sell_blocked(seen.limits, bottom)),
                (true, true),
                "a sell at the lower band fills nothing"
            );
            assert!(
                !buy_blocked(seen.limits, bottom),
                "a buy at the lower band is not held by it"
            );

            // A second that spans both limits holds both orders.
            let both = range(102, 55_000, 45_000);
            let seen = observe(&mut day, both);
            assert_eq!(
                (
                    buy_blocked(seen.limits, both),
                    sell_blocked(seen.limits, both)
                ),
                (true, true)
            );
        }
    }

    /// The design's fourth named case: resuming off the band. A lock at the
    /// band holds every buy until a second prints off it, and the band never
    /// narrows back.
    #[test]
    fn trading_resumes_off_the_band_and_the_band_never_narrows() {
        for flex in FlexReading::ALL {
            let mut day = band(RS_1000, flex);
            for ts in 0..5 {
                let lock = at(ts, 110_000);
                assert!(
                    buy_blocked(observe(&mut day, lock).limits, lock),
                    "second {ts} is locked"
                );
            }
            let off = at(5, 109_995);
            let seen = observe(&mut day, off);
            assert_eq!(
                (seen.at_upper, buy_blocked(seen.limits, off)),
                (false, false),
                "one tick off the band, a buy fills"
            );

            // After a flex the old limit holds nothing, and a fall back does not
            // narrow the band again.
            let flexed = observe(&mut day, at(6, 111_000)).limits;
            assert_eq!(flexed, limits(90_000, 115_000));
            let back = at(7, RS_1000);
            assert_eq!(observe(&mut day, back).limits, flexed);
            let old_top = at(8, 110_000);
            let seen = observe(&mut day, old_top);
            assert_eq!(
                (seen.at_upper, buy_blocked(seen.limits, old_top)),
                (false, false)
            );
            assert_eq!(limits_at(&day, 1_000_000), flexed);
        }
    }

    /// The design's fifth named case: a split day. R-BAND-1 takes the adjusted
    /// Zerodha close of D-1 times the ratio measured inside D's session, so the
    /// split, and any adjustment after it, cancels.
    #[test]
    fn a_split_day_takes_the_adjusted_close_times_the_session_ratio_inside_one_session() {
        // D-1 closed at Rs 2,000.00 raw; a 1:2 split goes ex on D. Zerodha's
        // history is back-adjusted, so its D-1 close reads Rs 1,000.00, and its
        // first minute of D agrees with GDFL's raw print: rho = 1.
        let split = Base::close_times_ratio(
            ExDay::Ordinary,
            100_000,
            SessionRatio {
                raw: 101_000,
                adjusted: 101_000,
            },
        )
        .expect("the split day has an R-BAND-1 base");
        assert_eq!(
            (split.reading(), split.paise()),
            (BaseReading::CloseTimesRatio, 100_000)
        );
        // A later adjustment by 5 rescales Zerodha's whole series; the ratio
        // measured in D's session (5) rescales it back.
        let later = Base::close_times_ratio(
            ExDay::Ordinary,
            20_000,
            SessionRatio {
                raw: 101_000,
                adjusted: 20_200,
            },
        )
        .expect("an R-BAND-1 base under a later adjustment");
        assert_eq!(later.paise(), split.paise());
        for flex in FlexReading::ALL {
            let mut day = DayBand::new(split, flex);
            for (ts, price) in [(0, 101_000), (1, 95_000), (2, 105_000), (3, 100_500)] {
                let seen = observe(&mut day, at(ts, price));
                assert_eq!(
                    (seen.up, seen.down),
                    (None, None),
                    "the post-split session needs no flex"
                );
                assert_eq!(seen.limits, limits(90_000, 110_000));
            }
        }
        // R-BAND-2 stands on D-1's raw last trade, in the old basis. The first
        // post-split print refutes the design's reading; SPEC's reading flexes
        // until it holds it. That band is the old basis's, and the headline
        // takes the worse lane per trade (D-8210; docs/06-limits.md).
        let old = base(200_000);
        let mut design = DayBand::new(old, FlexReading::FiveThreeTwo);
        assert_eq!(
            design.observe(at(0, 101_000)),
            Err(Refusal::PastTheLastFlex {
                ts: 0,
                side: BandSide::Lower,
                price: 101_000
            })
        );
        let mut spec = DayBand::new(old, FlexReading::FivePerStep);
        let seen = observe(&mut spec, at(0, 101_000));
        assert_eq!(
            seen.down,
            Some(Flex {
                from_ppm: 100_000,
                to_ppm: 500_000
            })
        );
        assert_eq!(seen.limits, limits(100_000, 220_000));
    }

    /// The design's sixth named case: a demerger ex-day is skipped under both
    /// readings, by name.
    #[test]
    fn a_demerger_ex_day_is_skipped_under_both_readings() {
        let ratio = SessionRatio {
            raw: 50_000,
            adjusted: 50_000,
        };
        assert_eq!(
            Base::close_times_ratio(ExDay::Demerger, RS_1000, ratio),
            Err(Refusal::DemergerExDay)
        );
        assert_eq!(
            Base::last_trade_before(ExDay::Demerger, RS_1000),
            Err(Refusal::DemergerExDay)
        );
        assert!(
            Refusal::DemergerExDay
                .to_string()
                .contains("demerger goes ex")
        );
        // The same prices on an ordinary day stand.
        assert!(Base::close_times_ratio(ExDay::Ordinary, RS_1000, ratio).is_ok());
        assert!(Base::last_trade_before(ExDay::Ordinary, RS_1000).is_ok());
    }

    /// The design's seventh named case: the first print of the day is already
    /// at the band, a gap up to the limit (and the gap down), under both bases.
    #[test]
    fn a_first_print_of_the_day_already_at_the_band() {
        let r1 = Base::close_times_ratio(
            ExDay::Ordinary,
            RS_1000,
            SessionRatio {
                raw: 110_000,
                adjusted: 110_000,
            },
        )
        .expect("an R-BAND-1 base");
        for start in [r1, base(RS_1000)] {
            for flex in FlexReading::ALL {
                let mut up = DayBand::new(start, flex);
                let open = at(33_300, 110_000);
                let seen = observe(&mut up, open);
                assert_eq!(
                    (seen.at_upper, seen.up),
                    (true, None),
                    "the open at the band flexes nothing"
                );
                assert!(
                    buy_blocked(seen.limits, open),
                    "a buy at the open fills nothing"
                );
                let mut down = DayBand::new(start, flex);
                let open = at(33_300, 90_000);
                let seen = observe(&mut down, open);
                assert_eq!((seen.at_lower, seen.down), (true, None));
                assert!(
                    sell_blocked(seen.limits, open),
                    "a sell at the open fills nothing"
                );
            }
        }
    }

    /// THE BAR's gaps: a no-trade gap, then a print past several flexes, takes
    /// them all in its one second; an opening gap inside the range moves
    /// nothing.
    #[test]
    fn a_no_trade_gap_then_a_print_past_several_flexes_takes_them_in_one_second() {
        let mut spec = band(RS_1000, FlexReading::FivePerStep);
        observe(&mut spec, at(33_300, RS_1000));
        let seen = observe(&mut spec, at(36_900, 127_000));
        assert_eq!(
            seen.up,
            Some(Flex {
                from_ppm: 100_000,
                to_ppm: 300_000
            }),
            "four steps of 5% at once"
        );
        assert_eq!(seen.limits.upper, 130_000);
        assert_eq!(
            limits_at(&spec, 36_899).upper,
            110_000,
            "the gap itself flexed nothing"
        );

        let mut design = band(RS_1000, FlexReading::FiveThreeTwo);
        let seen = observe(&mut design, at(36_900, 119_500));
        assert_eq!(
            seen.up,
            Some(Flex {
                from_ppm: 100_000,
                to_ppm: 200_000
            }),
            "the design's three at once"
        );

        for flex in FlexReading::ALL {
            let mut day = band(RS_1000, flex);
            let seen = observe(&mut day, at(33_300, 107_000));
            assert_eq!(
                (seen.up, seen.down, seen.limits),
                (None, None, limits(90_000, 110_000))
            );
        }
    }

    /// One second past both limits flexes both sides; under the design's
    /// reading a side past its last flex refuses, upper or lower.
    #[test]
    fn a_second_past_both_limits_flexes_both_sides() {
        let mut spec = band(RS_1000, FlexReading::FivePerStep);
        let seen = observe(&mut spec, range(5, 113_000, 87_000));
        let flex = Some(Flex {
            from_ppm: 100_000,
            to_ppm: 150_000,
        });
        assert_eq!((seen.up, seen.down), (flex, flex));
        assert_eq!(seen.limits, limits(85_000, 115_000));

        let mut design = band(RS_1000, FlexReading::FiveThreeTwo);
        assert_eq!(
            design.observe(range(5, 125_000, 95_000)),
            Err(Refusal::PastTheLastFlex {
                ts: 5,
                side: BandSide::Upper,
                price: 125_000
            })
        );
        let mut design = band(RS_1000, FlexReading::FiveThreeTwo);
        assert_eq!(
            design.observe(range(6, 100_000, 75_000)),
            Err(Refusal::PastTheLastFlex {
                ts: 6,
                side: BandSide::Lower,
                price: 75_000
            })
        );
    }

    /// The band at an earlier second is the band as it stood then, whatever
    /// was observed since.
    #[test]
    fn the_band_answers_an_earlier_second_as_it_stood_then() {
        let mut day = band(RS_1000, FlexReading::FivePerStep);
        for sec in [
            at(10, 112_000),
            at(15, 88_000),
            at(20, 117_000),
            at(25, 100_000),
        ] {
            observe(&mut day, sec);
        }
        for (ts, want) in [
            (-5, limits(90_000, 110_000)),
            (9, limits(90_000, 110_000)),
            (10, limits(90_000, 115_000)),
            (14, limits(90_000, 115_000)),
            (15, limits(85_000, 115_000)),
            (19, limits(85_000, 115_000)),
            (20, limits(85_000, 120_000)),
            (1_000, limits(85_000, 120_000)),
        ] {
            assert_eq!(limits_at(&day, ts), want, "second {ts}");
        }
        // Observing a second again changes nothing it already holds, and a
        // wider range in the same second flexes in that second.
        let again = observe(&mut day, at(25, 100_000));
        assert_eq!((again.up, again.down), (None, None));
        let wider = observe(&mut day, range(25, 121_000, 100_000));
        assert_eq!(wider.limits.upper, 125_000);
        assert_eq!(limits_at(&day, 24).upper, 120_000);
    }

    /// Every boundary on each side and under each reading: one paisa inside,
    /// at the limit, one paisa past.
    #[test]
    fn a_print_one_paisa_inside_at_and_past_each_limit() {
        for flex in FlexReading::ALL {
            for (side, limit, toward) in
                [(BandSide::Upper, 110_000, 1), (BandSide::Lower, 90_000, -1)]
            {
                for (shift, at_band, flexed) in
                    [(-1, false, false), (0, true, false), (1, false, true)]
                {
                    let mut day = band(RS_1000, flex);
                    let seen = observe(&mut day, at(0, limit + shift * toward));
                    let (at_side, flex_side) = match side {
                        BandSide::Upper => (seen.at_upper, seen.up),
                        BandSide::Lower => (seen.at_lower, seen.down),
                    };
                    assert_eq!(
                        at_side, at_band,
                        "{flex:?} {side:?} shift {shift}: at the band"
                    );
                    assert_eq!(
                        flex_side.is_some(),
                        flexed,
                        "{flex:?} {side:?} shift {shift}: flexed"
                    );
                }
            }
        }
    }

    /// Limits round inward to the paisa: the upper down, the lower up, the
    /// narrowest band the rule allows (OP-12, "the strict way").
    #[test]
    fn the_limits_round_inward_to_the_paisa() {
        // 10% of Rs 100.05 is Rs 10.005: the band is Rs 90.05 to Rs 110.05,
        // where the exact one is Rs 90.045 to Rs 110.055.
        assert_eq!(base(10_005).first_limits(), limits(9_005, 11_005));
        assert_eq!(base(19).first_limits(), limits(18, 20));
        assert_eq!(
            base(1).first_limits(),
            limits(1, 1),
            "a one-paisa base has no room either way"
        );
        // A flexed width rounds the same way: 15% of 10,005 is 1,500.75.
        let mut day = band(10_005, FlexReading::FivePerStep);
        assert_eq!(observe(&mut day, at(0, 11_006)).limits.upper, 11_505);
        assert_eq!(observe(&mut day, at(1, 9_004)).limits.lower, 8_505);
    }

    /// Each reading's base: R-BAND-2 as given; R-BAND-1 snapped to the paisa
    /// half-up, once, with the feeds' gap inside the ratio.
    #[test]
    fn the_base_of_each_reading() {
        let r2 = base(12_345);
        assert_eq!(
            (r2.reading(), r2.paise()),
            (BaseReading::LastTradeBefore, 12_345)
        );
        let r1 = |prev, raw, adjusted| {
            Base::close_times_ratio(ExDay::Ordinary, prev, SessionRatio { raw, adjusted })
                .map(Base::paise)
        };
        assert_eq!(r1(3, 1, 2), Ok(2), "1.5 rounds half-up to 2");
        assert_eq!(r1(5, 1, 4), Ok(1), "1.25 rounds to 1");
        assert_eq!(r1(7, 3, 2), Ok(11), "10.5 rounds half-up to 11");
        assert_eq!(
            r1(RS_1000, 101_050, 101_000),
            Ok(100_050),
            "the feeds' 5 bp gap moves the base 5 bp"
        );
        let gapped = Base::close_times_ratio(
            ExDay::Ordinary,
            RS_1000,
            SessionRatio {
                raw: 101_050,
                adjusted: 101_000,
            },
        )
        .expect("an R-BAND-1 base");
        assert_eq!(gapped.first_limits(), limits(90_045, 110_055));
    }

    /// Every refusal, by name, and a broken band keeps refusing from its
    /// second while its earlier answers stand.
    #[test]
    fn every_refusal_is_named() {
        let ratio = |raw, adjusted| SessionRatio { raw, adjusted };
        let not_positive = |what, value| Err(Refusal::NotPositive { what, value });
        assert_eq!(
            Base::close_times_ratio(ExDay::Ordinary, 0, ratio(1, 1)),
            not_positive("the adjusted close of D-1", 0)
        );
        assert_eq!(
            Base::close_times_ratio(ExDay::Ordinary, 1, ratio(-1, 1)),
            not_positive("the session ratio's raw price", -1)
        );
        assert_eq!(
            Base::close_times_ratio(ExDay::Ordinary, 1, ratio(1, 0)),
            not_positive("the session ratio's adjusted price", 0)
        );
        assert_eq!(
            Base::close_times_ratio(ExDay::Ordinary, 1, ratio(1, 3)),
            not_positive("the base price", 0),
            "a third of a paisa snaps to no base"
        );
        assert_eq!(
            Base::last_trade_before(ExDay::Ordinary, -7),
            not_positive("the last trade of D-1", -7)
        );
        assert_eq!(
            Base::close_times_ratio(ExDay::Ordinary, i64::MAX, ratio(i64::MAX, 1)),
            Err(Refusal::BaseOutOfRange {
                reading: BaseReading::CloseTimesRatio
            }),
            "a base past i64"
        );
        assert_eq!(
            Base::last_trade_before(ExDay::Ordinary, i64::MAX),
            Err(Refusal::BaseOutOfRange {
                reading: BaseReading::LastTradeBefore
            }),
            "a base whose first band is past i64"
        );

        let mut day = band(RS_1000, FlexReading::FiveThreeTwo);
        for bad in [range(1, 100, 0), range(1, 99, 100), range(1, -5, -6)] {
            assert_eq!(
                day.observe(bad),
                Err(Refusal::BadPrints {
                    ts: 1,
                    high: bad.high,
                    low: bad.low
                })
            );
        }
        observe(&mut day, at(5, RS_1000));
        assert_eq!(
            day.observe(at(4, RS_1000)),
            Err(Refusal::OutOfOrder { ts: 4, last: 5 })
        );
        assert_eq!(
            limits_at(&day, 5),
            limits(90_000, 110_000),
            "a refused second changes nothing"
        );
        assert_eq!(
            day.near(at(5, RS_1000), -1),
            Err(Refusal::NegativeStep { step: -1 })
        );
        assert_eq!(
            day.near(range(5, 1, 2), 1),
            Err(Refusal::BadPrints {
                ts: 5,
                high: 1,
                low: 2
            })
        );

        let refuted = Refusal::PastTheLastFlex {
            ts: 40,
            side: BandSide::Upper,
            price: 130_000,
        };
        assert_eq!(day.observe(at(40, 130_000)), Err(refuted));
        assert_eq!(
            day.observe(at(41, RS_1000)),
            Err(refuted),
            "a broken band stays broken"
        );
        assert_eq!(
            day.observe(at(39, RS_1000)),
            Err(Refusal::OutOfOrder { ts: 39, last: 40 })
        );
        assert_eq!(
            limits_at(&day, 39),
            limits(90_000, 110_000),
            "its earlier answers stand"
        );
        assert_eq!(day.limits(40), Err(refuted));
        assert_eq!(day.limits(41), Err(refuted));
        assert_eq!(day.near(at(41, RS_1000), 5), Err(refuted));
        assert_eq!(day.near(at(39, RS_1000), 5), Ok(false));

        let texts = [
            (Refusal::DemergerExDay, "demerger"),
            (
                Refusal::NotPositive {
                    what: "the last trade of D-1",
                    value: 0,
                },
                "the last trade of D-1 is 0 paise",
            ),
            (
                Refusal::BaseOutOfRange {
                    reading: BaseReading::CloseTimesRatio,
                },
                "the R-BAND-1 base",
            ),
            (
                Refusal::BadPrints {
                    ts: 1,
                    high: 2,
                    low: 3,
                },
                "low 3 and high 2",
            ),
            (
                Refusal::OutOfOrder { ts: 4, last: 5 },
                "second 5 was already observed",
            ),
            (Refusal::NegativeStep { step: -1 }, "-1 paise is negative"),
            (refuted, "130000 is past the Upper limit's last flex"),
            (
                Refusal::OutOfRange {
                    ts: 0,
                    side: BandSide::Lower,
                    price: 7,
                },
                "the Lower flex 7 needs",
            ),
        ];
        for (why, words) in texts {
            let text = why.to_string();
            assert!(text.contains(words), "{text:?} names {words:?}");
        }
    }

    /// THE BAR's `i128` guards: a flex whose half-width is past `u64` parts per
    /// million, or whose limit is past paise `i64`, is refused by name, never
    /// wrapped or saturated.
    #[test]
    fn the_i128_guards_refuse_a_band_past_the_types() {
        let mut tiny = band(1, FlexReading::FivePerStep);
        assert_eq!(
            tiny.observe(at(0, 1_000_000_000_000_000_000)),
            Err(Refusal::OutOfRange {
                ts: 0,
                side: BandSide::Upper,
                price: 1_000_000_000_000_000_000
            }),
            "a print 10^18 times a one-paisa base needs a half-width past u64"
        );
        let big = 1_i64 << 62;
        let mut wide = band(big, FlexReading::FivePerStep);
        assert_eq!(
            wide.observe(at(0, i64::MAX)),
            Err(Refusal::OutOfRange {
                ts: 0,
                side: BandSide::Upper,
                price: i64::MAX
            }),
            "doubling 2^62 is one paisa past i64"
        );
        let mut held = band(big, FlexReading::FivePerStep);
        let seen = observe(&mut held, at(0, big + big / 5));
        assert_eq!(
            seen.up,
            Some(Flex {
                from_ppm: 100_000,
                to_ppm: 200_000
            })
        );
        let mut design = band(big, FlexReading::FiveThreeTwo);
        assert_eq!(
            design.observe(at(0, i64::MAX)),
            Err(Refusal::PastTheLastFlex {
                ts: 0,
                side: BandSide::Upper,
                price: i64::MAX
            }),
            "the design's reading is refuted before its limit could leave i64"
        );
    }

    /// The lane test: within one price step of a limit, or past one.
    #[test]
    fn near_is_within_one_step_of_a_limit() {
        let day = band(RS_1000, FlexReading::FivePerStep);
        let step = 5;
        assert_eq!(
            day.near(at(0, 109_995), step),
            Ok(true),
            "a high one step under the upper limit"
        );
        assert_eq!(
            day.near(range(0, RS_1000, 90_005), step),
            Ok(true),
            "a low one step over the lower limit"
        );
        assert_eq!(
            day.near(range(0, 109_994, 90_006), step),
            Ok(false),
            "both a paisa further in"
        );
        assert_eq!(
            day.near(at(0, 110_000), 0),
            Ok(true),
            "step zero: at the band"
        );
        assert_eq!(
            day.near(at(0, 109_999), 0),
            Ok(false),
            "step zero: one paisa in"
        );
        assert_eq!(
            day.near(at(0, 111_000), 0),
            Ok(true),
            "past a limit not yet observed"
        );
    }

    /// The label, the rule word, the readings' names and the constants they
    /// are built from.
    #[test]
    fn the_label_the_rule_word_and_the_reading_names() {
        assert_eq!(LABEL, "band computed");
        assert_eq!(BAND_RULE, "fno-band-v1");
        assert_eq!(
            BaseReading::ALL.map(BaseReading::id),
            ["R-BAND-1", "R-BAND-2"]
        );
        assert_eq!(
            FlexReading::ALL.map(FlexReading::id),
            [
                "flex 5% a step (SPEC 10.2)",
                "flex 5%, then 3%, then 2% (FNO-BAND design)"
            ]
        );
        assert_eq!((RANGE_PPM, SPEC_FLEX_PPM), (100_000, 50_000), "10% and 5%");
        assert_eq!(
            DESIGN_WIDTHS_PPM,
            [
                RANGE_PPM + 50_000,
                RANGE_PPM + 50_000 + 30_000,
                RANGE_PPM + 50_000 + 30_000 + 20_000
            ],
            "10% then 5%, then 3%, then 2%"
        );
        let day = band(RS_1000, FlexReading::FiveThreeTwo);
        assert_eq!(
            (day.base(), day.flex()),
            (base(RS_1000), FlexReading::FiveThreeTwo)
        );
        assert_eq!(limits(1, 2).pair(), (1, 2));
    }

    /// The bench's quantiles take the nearest rank.
    #[test]
    fn quantiles_take_the_nearest_rank() {
        assert_eq!(quantile(&[], 500), None);
        assert_eq!(quantile(&[1, 2], 1_001), None);
        assert_eq!(quantile(&[5], 0), Some(5));
        let hundred: Vec<u64> = (1..=100).rev().collect();
        assert_eq!(quantile(&hundred, 500), Some(50));
        assert_eq!(quantile(&hundred, 990), Some(99));
        assert_eq!(quantile(&hundred, 1_000), Some(100));
        assert_eq!(quantile(&hundred, 1), Some(1));
    }

    /// The bench's fixture, at a tiny size: every second timed once each way,
    /// one flex a side, nothing refused; and the one refusal it passes on.
    #[test]
    fn the_bench_fixture_times_every_second_it_walks() {
        for flex in FlexReading::ALL {
            let (mut day, seconds) = bench_day(flex, 50);
            assert_eq!(
                day.base(),
                base(RS_1000),
                "the bench base is R-BAND-2 at Rs 1,000.00"
            );
            assert_eq!(seconds.len(), 50);
            // After the first two, every second stays inside the flexed band.
            for sec in &seconds[2..] {
                assert!(sec.low >= 86_000, "{sec:?}");
                assert!(sec.high < 114_000, "{sec:?}");
                assert!(sec.high >= sec.low, "{sec:?}");
            }
            let samples = measure_checks(&mut day, &seconds).expect("the fixture is a valid day");
            assert_eq!(
                (samples.check_ns.len(), samples.lookback_ns.len()),
                (50, 50)
            );
            assert_eq!((samples.flexes, samples.refused), (2, 0));
            assert_eq!(limits_at(&day, 49), limits(85_000, 115_000));
            let (mut empty, none) = bench_day(flex, 0);
            assert_eq!(
                measure_checks(&mut empty, &none).map(|s| s.check_ns.len()),
                Ok(0)
            );
        }
        let (mut day, _) = bench_day(FlexReading::FivePerStep, 0);
        assert_eq!(
            measure_checks(&mut day, &[at(5, RS_1000), at(4, RS_1000)]),
            Err(Refusal::OutOfOrder { ts: 4, last: 5 })
        );
    }

    /// No look-ahead (SPEC FIB-I1): on seeded random sessions, the band at
    /// every second up to a cut is the same whatever the seconds after the cut
    /// print.
    #[test]
    fn no_later_second_moves_an_earlier_band() {
        for seed in 1..=150 {
            let mut rng = Rng::seeded(seed);
            let start = 1_000 + rng.below(400_000);
            let seconds = session(&mut rng, start, 160);
            let cut = usize::try_from(rng.below(160)).expect("an index");
            let mut other = seconds[..=cut].to_vec();
            let mut future = Rng::seeded(seed + 10_000);
            let last = seconds[cut].ts;
            other.extend(
                session(&mut future, start, 160 - cut - 1)
                    .into_iter()
                    .map(|sec| range(last + sec.ts - 33_300, sec.high, sec.low)),
            );
            for flex in FlexReading::ALL {
                let mut one = band(start, flex);
                let mut two = band(start, flex);
                for sec in &seconds {
                    let _ = one.observe(*sec);
                }
                for sec in &other {
                    let _ = two.observe(*sec);
                }
                for sec in &seconds[..=cut] {
                    for ts in [sec.ts - 1, sec.ts] {
                        assert_eq!(
                            one.limits(ts),
                            two.limits(ts),
                            "seed {seed} {flex:?} second {ts}"
                        );
                    }
                }
            }
        }
    }

    /// On seeded random sessions: the band holds every print it observed in
    /// that print's second, answers that second as observe did, and never
    /// narrows; a broken band refuses every later second the same way.
    #[test]
    fn the_band_holds_every_print_it_observed_and_never_narrows() {
        let mut refuted = 0;
        for seed in 1..=150 {
            let mut rng = Rng::seeded(seed);
            let start = 1_000 + rng.below(400_000);
            for flex in FlexReading::ALL {
                let mut day = band(start, flex);
                let mut before = day.base().first_limits();
                let mut broke: Option<Refusal> = None;
                for sec in session(&mut rng, start, 200) {
                    match (day.observe(sec), broke) {
                        (Ok(seen), None) => {
                            assert!(seen.limits.lower <= sec.low, "seed {seed}: holds the low");
                            assert!(sec.high <= seen.limits.upper, "seed {seed}: holds the high");
                            assert!(
                                seen.limits.lower <= before.lower,
                                "seed {seed}: never narrows"
                            );
                            assert!(
                                seen.limits.upper >= before.upper,
                                "seed {seed}: never narrows"
                            );
                            assert_eq!(day.limits(sec.ts), Ok(seen.limits));
                            before = seen.limits;
                        }
                        (Err(why), None) => {
                            refuted += 1;
                            let past = |side, price| Refusal::PastTheLastFlex {
                                ts: sec.ts,
                                side,
                                price,
                            };
                            assert!(
                                [
                                    past(BandSide::Upper, sec.high),
                                    past(BandSide::Lower, sec.low)
                                ]
                                .contains(&why),
                                "seed {seed}: only a reading with a last flex breaks: {why}"
                            );
                            broke = Some(why);
                        }
                        (outcome, Some(why)) => assert_eq!(outcome, Err(why), "seed {seed}"),
                    }
                }
            }
        }
        assert!(
            refuted > 0,
            "the sessions refute the design's reading at least once"
        );
    }

    /// On seeded random sessions, each flex is the fewest steps of its reading
    /// that hold the print: the rung below the new half-width would not.
    #[test]
    fn each_flex_is_the_fewest_steps_that_hold_the_print() {
        let mut flexes = 0;
        for seed in 1..=150 {
            let mut rng = Rng::seeded(seed);
            let start = 1_000 + rng.below(400_000);
            for flex in FlexReading::ALL {
                let mut day = band(start, flex);
                for sec in session(&mut rng, start, 200) {
                    let Ok(seen) = day.observe(sec) else { break };
                    for (side, moved, print) in [
                        (BandSide::Upper, seen.up, sec.high),
                        (BandSide::Lower, seen.down, sec.low),
                    ] {
                        let Some(moved) = moved else { continue };
                        flexes += 1;
                        let below = limit_of(start, width_below(flex, moved.to_ppm), side);
                        let short = match side {
                            BandSide::Upper => below < print,
                            BandSide::Lower => below > print,
                        };
                        assert!(
                            short,
                            "seed {seed} {flex:?} {side:?}: one rung less would hold {print}"
                        );
                        assert!(moved.to_ppm > moved.from_ppm);
                    }
                }
            }
        }
        assert!(flexes > 100, "the sessions flex: {flexes}");
    }

    /// While the design's reading holds, its band is never wider than SPEC's
    /// on the same prints: its ladder is never above SPEC's (D-8211).
    #[test]
    fn the_designs_band_is_never_wider_than_specs_while_it_holds() {
        for seed in 1..=150 {
            let mut rng = Rng::seeded(seed);
            let start = 1_000 + rng.below(400_000);
            let mut spec = band(start, FlexReading::FivePerStep);
            let mut design = band(start, FlexReading::FiveThreeTwo);
            for sec in session(&mut rng, start, 200) {
                let wide = spec.observe(sec).expect("SPEC's reading has no last flex");
                let Ok(narrow) = design.observe(sec) else {
                    break;
                };
                assert!(narrow.limits.upper <= wide.limits.upper, "seed {seed}");
                assert!(narrow.limits.lower >= wide.limits.lower, "seed {seed}");
            }
        }
    }

    /// Asking each second right after observing it, or observing the whole day
    /// first and asking in a shuffled order, gives the same answers.
    #[test]
    fn asking_in_any_order_gives_the_same_answers() {
        for seed in 1..=100 {
            let mut rng = Rng::seeded(seed);
            let start = 1_000 + rng.below(400_000);
            let seconds = session(&mut rng, start, 120);
            for flex in FlexReading::ALL {
                let mut walking = band(start, flex);
                let asked: Vec<_> = seconds
                    .iter()
                    .map(|sec| {
                        (
                            sec.ts,
                            walking.observe(*sec).and_then(|_| walking.limits(sec.ts)),
                        )
                    })
                    .collect();
                let mut whole = band(start, flex);
                for sec in &seconds {
                    let _ = whole.observe(*sec);
                }
                let mut order: Vec<usize> = (0..asked.len()).collect();
                for i in (1..order.len()).rev() {
                    let j = usize::try_from(rng.below(u64::try_from(i + 1).expect("a count")))
                        .expect("an index");
                    order.swap(i, j);
                }
                for i in order {
                    let (ts, answer) = asked[i];
                    assert_eq!(whole.limits(ts), answer, "seed {seed} {flex:?} second {ts}");
                }
            }
        }
    }

    /// The lane rule of PARTS FNO-BAND: a second no lane calls near (within
    /// one price step of a limit, or past one) has the same rule-4 outcome in
    /// every lane, so a trade made only of such seconds walks one lane.
    #[test]
    fn a_trade_far_from_every_band_walks_one_lane() {
        let (mut far, mut near) = (0, 0);
        for seed in 1..=100 {
            let mut rng = Rng::seeded(seed);
            let start = 1_000 + rng.below(400_000);
            let close = Base::close_times_ratio(
                ExDay::Ordinary,
                start,
                SessionRatio {
                    raw: 1_000_000 + rng.below(800),
                    adjusted: 1_000_000,
                },
            )
            .expect("an R-BAND-1 base");
            let mut lanes: Vec<DayBand> = [close, base(start)]
                .into_iter()
                .flat_map(|stand| FlexReading::ALL.map(|flex| DayBand::new(stand, flex)))
                .collect();
            let step = start / 2_000 + 1;
            for sec in session(&mut rng, start, 150) {
                // A lane the data refuted is no lane any more: FNO-RUN drops it,
                // so only the standing lanes are asked.
                let standing: Vec<(bool, Limits)> = lanes
                    .iter_mut()
                    .filter_map(|lane| {
                        let stands = lane.observe(sec).is_ok();
                        stands.then(|| {
                            (
                                lane.near(sec, step).expect("a standing lane answers near"),
                                limits_at(lane, sec.ts),
                            )
                        })
                    })
                    .collect();
                assert!(
                    standing.len() >= 2,
                    "SPEC's reading never refutes: both of its lanes stand"
                );
                if standing.iter().any(|&(close, _)| close) {
                    near += 1;
                    continue;
                }
                far += 1;
                for (_, limits) in standing {
                    assert!(
                        !buy_blocked(limits, sec),
                        "seed {seed}: no lane holds a buy"
                    );
                    assert!(
                        !sell_blocked(limits, sec),
                        "seed {seed}: no lane holds a sell"
                    );
                }
            }
        }
        assert!(far > 1_000, "{far} seconds far from every band");
        assert!(near > 100, "{near} seconds near a band");
    }
}
