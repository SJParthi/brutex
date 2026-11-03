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
            Self::NotPositive { what, value } => write!(
                f,
                "no band: {what} is {value} paise, not a positive price"
            ),
            Self::BaseOutOfRange { reading } => write!(
                f,
                "no band: the {} base or its first band does not fit paise i64",
                reading.id()
            ),
            Self::BadPrints { ts, high, low } => write!(
                f,
                "second {ts} refused: low {low} and high {high} are not a traded range"
            ),
            Self::OutOfOrder { ts, last } => write!(
                f,
                "second {ts} refused: second {last} was already observed"
            ),
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
    let rank = sorted.len().saturating_mul(usize::from(permille)).div_ceil(1_000);
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
