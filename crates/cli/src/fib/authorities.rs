//! The authorities a FIB run reads, each behind a trait with one lookup per
//! question, and the INTERIM implementations every run reads until the
//! new-work packages land (FIB-8; D-6442).
//!
//! # The traits
//!
//! | Trait | Asks | Interim |
//! |---|---|---|
//! | [`CalendarAuthority`] | is `(series, day)` tradable | the H5 formula on `pull::calendar` (below) |
//! | [`PoolAuthority`] | who is in the pool on `day`, and at which slot | the development pool |
//! | [`FactorAuthority`] | DATA-1's Zerodha/GDFL close pair for a stock-day | identity, development only |
//! | [`SecondsTrust`] | DATA-2's verdict on a stock-day's GDFL seconds | unchecked |
//! | [`OrderFlowAuthority`] | OF-1's signed window | absent: the row is constant |
//! | [`LandingAuthority`] | which landing a stored day came from, and PREC-10's breaks | one landing assumed |
//! | [`BasisVerdictAuthority`] | the per-stock-day basis verdict (RQ-4) | FM-F092's proof over the two above |
//! | [`TailWitness`] | did trades happen after a day's last Zerodha minute | unknown: `tail_unwitnessed` |
//! | [`IndexScreen`] | PREC-9's verdict on an index day | `index day unscreened` |
//! | [`ExpiryAuthority`] | the expiry stamps of a stock-day | absent: no expiry source loaded |
//!
//! # Every lookup is O(1)
//!
//! A calendar answer is a fixed-table walk inside `pull::calendar` (its own
//! documented bound); a pool slot is one read of a dense per-day array built
//! once per day in O(pool) ([`DaySlots`], FM-F086); every other interim answer
//! is a constant. Measured by the `fib` bench target's authority rows.
//!
//! # No `Default`, and every interim one is refused for `--out-store`
//!
//! No authority type implements `Default`: each is constructed by name and
//! carries an [`AuthorityVersion`] — its kind, the flag word that selected it,
//! its label and a digest of what it answers — which a run records as an
//! identity leaf. An interim implementation answers from a stand-in (the
//! survivorship-biased development pool, a factor of one, unchecked seconds,
//! no order flow, one assumed landing), so [`Authorities::refuse_for_out_store`]
//! refuses every one of them by name: a result written to the store must never
//! rest on a stand-in (`CLAUDE.md` §4: no fallback that hides a failure).
//!
//! # The new-work words
//!
//! `cal1`, `pit`, `data1`, `data2` and `of1` are routed to
//! [`super::wire::register`], which refuses each by name until FIB-10 builds
//! it: an authority that is not built is never quietly replaced by an interim
//! one.
//!
//! Measured, every lookup and record operation of this module: `crates/cli/benches/fib.rs` (FIB-8's rows, C-F8-01 to C-F8-10).

use std::fmt;

use brutex_core::blake3::Hasher;
use pull::calendar::{DayKind, Session, SpecialKind};
use vocab::fib_cube::{DIGEST_BYTES, OrderFlowWindow};

use super::wire;

// ------------------------------------------------------------- identity --

/// A dense series id: the point-in-time series id, or the development pool's
/// ordinal. Dense, so a per-day slot table is an array indexed by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SeriesId(pub u32);

/// What an authority answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuthorityKind {
    /// Whether a stock-day is tradable.
    Calendar,
    /// The pool and its slots.
    Pool,
    /// DATA-1's per-stock-day factor pair.
    Factor,
    /// DATA-2's per-stock-day seconds verdict.
    SecondsTrust,
    /// OF-1's signed order-flow windows.
    OrderFlow,
    /// Landings and PREC-10's basis breaks.
    Landing,
    /// The per-stock-day basis verdict (RQ-4).
    Basis,
    /// Whether trades happened after a day's last Zerodha minute.
    TailWitness,
    /// PREC-9's index-day census.
    IndexScreen,
    /// The expiry stamps.
    Expiry,
}

impl AuthorityKind {
    /// Every kind, in the order a run records them.
    pub const ALL: &[Self] = &[
        Self::Calendar,
        Self::Pool,
        Self::Factor,
        Self::SecondsTrust,
        Self::OrderFlow,
        Self::Landing,
        Self::Basis,
        Self::TailWitness,
        Self::IndexScreen,
        Self::Expiry,
    ];

    /// The kind's name and the flag that selects it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Calendar => "calendar",
            Self::Pool => "pool",
            Self::Factor => "factor",
            Self::SecondsTrust => "seconds",
            Self::OrderFlow => "order-flow",
            Self::Landing => "landing",
            Self::Basis => "basis",
            Self::TailWitness => "tail",
            Self::IndexScreen => "screen",
            Self::Expiry => "expiry",
        }
    }

    /// The interim implementation's flag word.
    #[must_use]
    pub const fn interim_word(self) -> &'static str {
        match self {
            Self::Calendar => "interim",
            Self::Pool => "dev",
            Self::Factor => "identity",
            Self::SecondsTrust => "unchecked",
            Self::OrderFlow => "absent",
            Self::Landing => "single-landing",
            Self::Basis => "fm-f092",
            Self::TailWitness => "unwitnessed",
            Self::IndexScreen => "unscreened",
            Self::Expiry => "unloaded",
        }
    }

    /// The new-work word that replaces the interim one, when a package owns it.
    #[must_use]
    pub const fn new_work_word(self) -> Option<&'static str> {
        match self {
            Self::Calendar => Some("cal1"),
            Self::Pool => Some("pit"),
            Self::Factor => Some("data1"),
            Self::SecondsTrust => Some("data2"),
            Self::OrderFlow => Some("of1"),
            Self::Landing | Self::Basis | Self::TailWitness | Self::IndexScreen | Self::Expiry => {
                None
            }
        }
    }
}

/// An implementation's name and version digest, recorded in the run identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityVersion {
    /// What it answers.
    pub kind: AuthorityKind,
    /// The flag word that selected it.
    pub word: &'static str,
    /// What it is, in words, as every banner prints it.
    pub label: &'static str,
    /// blake3 of the kind, the word, the label and what it answers.
    pub digest: [u8; DIGEST_BYTES],
    /// Whether it is an interim stand-in, refused for `--out-store`.
    pub interim: bool,
}

/// The digest of one implementation: a domain tag, then the kind, word and
/// label framed by length, then the table it answers from.
#[must_use]
pub fn version_digest(
    kind: AuthorityKind,
    word: &str,
    label: &str,
    table: &[u8],
) -> [u8; DIGEST_BYTES] {
    let mut hash = Hasher::new();
    hash.update(b"BTXFIBA1");
    for part in [
        kind.name().as_bytes(),
        word.as_bytes(),
        label.as_bytes(),
        table,
    ] {
        hash.update(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
        hash.update(part);
    }
    hash.finalize()
}

/// An interim implementation's version.
fn interim(kind: AuthorityKind, label: &'static str, table: &[u8]) -> AuthorityVersion {
    let word = kind.interim_word();
    AuthorityVersion {
        kind,
        word,
        label,
        digest: version_digest(kind, word, label, table),
        interim: true,
    }
}

// ------------------------------------------------------------- calendar --

/// Why a day is not tradable, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalendarCode {
    /// The exchange did not trade: a weekend or a holiday.
    Closed,
    /// Outside every sourced span of the calendar: no answer.
    Unmeasured,
    /// The exchange traded for a length no record states.
    LengthUnmeasured,
    /// An open session that is not the standard full session.
    NotFullSession,
    /// A special session (Muhurat, a weekend Budget or full session, a drill).
    SpecialSession,
    /// A market-wide halt.
    Halt,
    /// The 2021-02-24 systems outage (`pull::calendar::SYSTEMS_OUTAGE_DAY`).
    SystemsOutage,
}

impl CalendarCode {
    /// Every code, in counting order.
    pub const ALL: &[Self] = &[
        Self::Closed,
        Self::Unmeasured,
        Self::LengthUnmeasured,
        Self::NotFullSession,
        Self::SpecialSession,
        Self::Halt,
        Self::SystemsOutage,
    ];

    /// The code's counted name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Unmeasured => "calendar_unmeasured",
            Self::LengthUnmeasured => "session_length_unmeasured",
            Self::NotFullSession => "not_full_session",
            Self::SpecialSession => "special_session",
            Self::Halt => "halt",
            Self::SystemsOutage => "systems_outage",
        }
    }

    /// The code's position in [`Self::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Why a day was refused, and whether a live trader knew it before the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CalendarReason {
    /// The code.
    pub code: CalendarCode,
    /// The special session's kind, when the code is [`CalendarCode::SpecialSession`].
    pub special: Option<SpecialKind>,
    /// Whether the exclusion was known before the day (a holiday, a scheduled
    /// special session) or is hindsight (a halt, an outage, no answer). A
    /// hindsight exclusion's pre-halt decisions are shown as halt exposure
    /// (FM-F100).
    pub known_in_advance: bool,
}

impl fmt::Display for CalendarReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code.name())?;
        if let Some(kind) = self.special {
            write!(f, " ({})", kind.label())?;
        }
        if !self.known_in_advance {
            f.write_str(", known only in hindsight")?;
        }
        Ok(())
    }
}

/// The per-instrument session record of a stock-day (FM-F097).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InstrumentSession {
    /// No per-instrument record is loaded: `instrument_session_unchecked`,
    /// counted and shown on the banner.
    Unchecked,
    /// The record admits the stock on the day.
    Admitted,
    /// The record refuses the stock on the day, by name.
    Refused(&'static str),
}

impl InstrumentSession {
    /// The counted word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Unchecked => "instrument_session_unchecked",
            Self::Admitted => "instrument_session_admitted",
            Self::Refused(why) => why,
        }
    }
}

/// The market-wide verdict and the per-instrument record of one stock-day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tradable {
    /// The market-wide verdict.
    pub market: Result<(), CalendarReason>,
    /// The stock's own record.
    pub instrument: InstrumentSession,
}

impl Tradable {
    /// Whether the stock may trade on the day: the market admits it and its
    /// own record does not refuse it. `Unchecked` admits, labelled.
    #[must_use]
    pub const fn admits(&self) -> bool {
        self.market.is_ok() && !matches!(self.instrument, InstrumentSession::Refused(_))
    }
}

/// Whether a stock-day is tradable (operator rule h: a full calendar session
/// with no special session and no halt; FM-F097: and no per-instrument
/// refusal).
pub trait CalendarAuthority {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The verdict for `series` on IST epoch day `day`. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn tradable(&self, series: SeriesId, day: i64) -> Tradable;
    /// The market-wide verdict alone, for an instrument with no series (an
    /// index reference stamp). O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn market(&self, day: i64) -> Result<(), CalendarReason>;
    /// Whether the exchange traded at all on `day`: the calendar's own answer,
    /// which the previous traded session is read from. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn traded(&self, day: i64) -> bool;
}

/// The INTERIM calendar: the H5 formula on today's functions —
/// `pull::calendar::sourced_kind_of(day) == DayKind::Open(Session::full())`
/// and `special_session(day).is_none()` and `halt_on(day).is_none()` — with
/// the per-instrument record unchecked.
///
/// `sourced_kind_of` equals `kind_of` inside 2019-12-02..2026-09-04 and is
/// circular-sourced outside it, where `kind_of` alone answers `Unmeasured`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterimCalendar {
    version: AuthorityVersion,
}

/// The interim calendar's label.
pub const INTERIM_CALENDAR_LABEL: &str = "interim calendar: sourced_kind_of == full session, no \
     special session, no halt (H5); per-instrument sessions unchecked";

impl InterimCalendar {
    /// The interim calendar, its digest taken over its answer on every day of
    /// the calendar's sourced spans: an edit of `pull::calendar` changes it.
    #[must_use]
    pub fn new() -> Self {
        let mut table = Vec::new();
        let mut day = pull::calendar::SOURCED_FIRST_DAY;
        while day <= pull::calendar::SOURCED_LAST_DAY {
            table.extend_from_slice(&day.to_le_bytes());
            // An admitted day is the position past every code.
            let code = market_verdict(day)
                .err()
                .map_or(CalendarCode::ALL.len(), |reason| reason.code.index());
            table.extend_from_slice(&u64::try_from(code).unwrap_or(u64::MAX).to_le_bytes());
            day += 1;
        }
        Self {
            version: interim(AuthorityKind::Calendar, INTERIM_CALENDAR_LABEL, &table),
        }
    }
}

/// The H5 formula's market-wide answer for `day`: [`classify`] over
/// `pull::calendar`'s three answers for it.
#[must_use]
pub fn market_verdict(day: i64) -> Result<(), CalendarReason> {
    let halt = pull::calendar::halt_on(day).map(|_| {
        if day == pull::calendar::SYSTEMS_OUTAGE_DAY {
            CalendarCode::SystemsOutage
        } else {
            CalendarCode::Halt
        }
    });
    classify(
        pull::calendar::sourced_kind_of(day),
        pull::calendar::special_session(day).map(|special| special.kind),
        halt,
    )
}

/// The H5 formula over one day's calendar answers: a halt first (`halt`
/// names its code; the 2021-02-24 outage is both a halt and a special
/// session, and is named by its halt), then a special session, then the
/// session kind. A special
/// session is known in advance, an outage excepted; a halt, a short or
/// unsized session and no answer are known only in hindsight.
#[must_use]
pub fn classify(
    kind: DayKind,
    special: Option<SpecialKind>,
    halt: Option<CalendarCode>,
) -> Result<(), CalendarReason> {
    let hindsight = |code| CalendarReason {
        code,
        special: None,
        known_in_advance: false,
    };
    if let Some(code) = halt {
        return Err(hindsight(code));
    }
    if let Some(special) = special {
        return Err(CalendarReason {
            code: CalendarCode::SpecialSession,
            special: Some(special),
            known_in_advance: !matches!(special, SpecialKind::SystemsOutage),
        });
    }
    match kind {
        DayKind::Open(session) if session == Session::full() => Ok(()),
        DayKind::Open(_) => Err(hindsight(CalendarCode::NotFullSession)),
        DayKind::Closed => Err(CalendarReason {
            code: CalendarCode::Closed,
            special: None,
            known_in_advance: true,
        }),
        DayKind::OpenLengthUnmeasured => Err(hindsight(CalendarCode::LengthUnmeasured)),
        DayKind::Unmeasured => Err(hindsight(CalendarCode::Unmeasured)),
    }
}

impl CalendarAuthority for InterimCalendar {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn tradable(&self, _series: SeriesId, day: i64) -> Tradable {
        Tradable {
            market: market_verdict(day),
            instrument: InstrumentSession::Unchecked,
        }
    }

    fn market(&self, day: i64) -> Result<(), CalendarReason> {
        market_verdict(day)
    }

    fn traded(&self, day: i64) -> bool {
        matches!(
            pull::calendar::exchange_kind_of(day),
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured
        )
    }
}

/// Counts per IST year and per position, appended in the order years are met.
/// Days stream in order, so the year asked is almost always the last one; any
/// other is a walk of the years already met, at most the span's years.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YearCounts<const N: usize> {
    years: Vec<(u16, [u64; N])>,
}

impl<const N: usize> YearCounts<N> {
    /// No year counted yet.
    #[must_use]
    pub const fn new() -> Self {
        Self { years: Vec::new() }
    }

    /// Counts one at `position` in `year`. A position past `N` is ignored.
    pub fn add(&mut self, year: u16, position: usize) {
        let at = match self.years.iter().rposition(|(seen, _)| *seen == year) {
            Some(at) => at,
            None => {
                self.years.push((year, [0; N]));
                self.years.len() - 1
            }
        };
        if let Some(count) = self
            .years
            .get_mut(at)
            .and_then(|(_, counts)| counts.get_mut(position))
        {
            *count = count.saturating_add(1);
        }
    }

    /// The count at `position` in `year`.
    #[must_use]
    pub fn get(&self, year: u16, position: usize) -> u64 {
        self.years
            .iter()
            .find(|(seen, _)| *seen == year)
            .and_then(|(_, counts)| counts.get(position))
            .copied()
            .unwrap_or(0)
    }

    /// Every year met, with its counts, in the order met.
    #[must_use]
    pub fn years(&self) -> &[(u16, [u64; N])] {
        &self.years
    }

    /// The count at `position` over every year.
    #[must_use]
    pub fn total(&self, position: usize) -> u64 {
        self.years
            .iter()
            .filter_map(|(_, counts)| counts.get(position))
            .fold(0_u64, |sum, count| sum.saturating_add(*count))
    }
}

/// The IST civil year of an epoch day, or `None` where the calendar cannot
/// name one.
#[must_use]
pub fn year_of(day: i64) -> Option<u16> {
    u32::try_from(day)
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
        .map(pull::session::Day::year)
}

// ----------------------------------------------------------------- pool --

/// One member of the pool on one day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// The dense series id.
    pub series_id: SeriesId,
    /// The symbol the series traded under on that day: the store key it loads
    /// from.
    pub symbol_on_day: String,
    /// Whether a Zerodha series directory exists for it.
    pub zerodha_series_present: bool,
    /// Whether it is in today's F&O list.
    pub in_current_list: bool,
}

/// A per-day dense slot table over dense series ids (FM-F086): built once per
/// day in O(pool), and each [`DaySlots::slot_of`] is one bounds-checked array
/// read, O(1) worst case.
/// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaySlots {
    slots: Vec<Option<u16>>,
}

impl DaySlots {
    /// The table of one day's members, in pool order.
    ///
    /// # Errors
    ///
    /// `pool_exceeds_slots` when the pool holds more members than a `u16`
    /// slot can name, and `series_listed_twice` when one series id is a member
    /// twice.
    pub fn build(members: &[Member]) -> Result<Self, String> {
        let widest = members
            .iter()
            .map(|member| member.series_id.0)
            .max()
            .map_or(0, |id| usize::try_from(id).unwrap_or(usize::MAX));
        let mut slots = vec![None; widest.saturating_add(1)];
        for (slot, member) in members.iter().enumerate() {
            let slot = u16::try_from(slot).map_err(|_| {
                format!(
                    "pool_exceeds_slots: {} members, past the {} a u16 slot names",
                    members.len(),
                    u16::MAX
                )
            })?;
            let at = usize::try_from(member.series_id.0).unwrap_or(usize::MAX);
            match slots.get_mut(at) {
                Some(entry @ None) => *entry = Some(slot),
                _ => {
                    return Err(format!(
                        "series_listed_twice: series {} ({}) is a member twice",
                        member.series_id.0, member.symbol_on_day
                    ));
                }
            }
        }
        Ok(Self { slots })
    }

    /// The slot of `series`, or `None` when it is not a member. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    #[must_use]
    pub fn slot_of(&self, series: SeriesId) -> Option<u16> {
        usize::try_from(series.0)
            .ok()
            .and_then(|at| self.slots.get(at))
            .copied()
            .flatten()
    }
}

/// Who is in the rank pool on a day, and at which slot.
pub trait PoolAuthority {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The members on IST epoch day `day`, in slot order.
    fn members(&self, day: i64) -> &[Member];
    /// The slot of `series` on `day`, or `None`. O(1) worst case.
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn slot_of(&self, series: SeriesId, day: i64) -> Option<u16>;
    /// The member whose symbol is `symbol` on `day`.
    fn member_named(&self, symbol: &str, day: i64) -> Option<&Member> {
        self.members(day)
            .iter()
            .find(|member| member.symbol_on_day == symbol)
    }
}

/// The development pool's label.
pub const DEVELOPMENT_POOL_LABEL: &str = "survivorship-biased development pool (2026-08-01 \
     snapshot): core::universe::FNO_UNDERLYINGS less FNO_INDEX_UNDERLYINGS; renames not stitched";

/// The INTERIM pool: the development pool, `core::universe::FNO_UNDERLYINGS`
/// less `FNO_INDEX_UNDERLYINGS`, the same every day, each series id the
/// share's ordinal in that list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevelopmentPool {
    version: AuthorityVersion,
    members: Vec<Member>,
    slots: DaySlots,
}

impl DevelopmentPool {
    /// The development pool; `present` answers whether a Zerodha series
    /// directory exists for a symbol.
    ///
    /// # Errors
    ///
    /// Every [`DaySlots::build`] refusal (none for the shipped list).
    pub fn new(present: &dyn Fn(&str) -> bool) -> Result<Self, String> {
        let mut members = Vec::new();
        let mut table = Vec::new();
        for symbol in development_symbols() {
            let series_id = SeriesId(u32::try_from(members.len()).unwrap_or(u32::MAX));
            table.extend_from_slice(
                &u64::try_from(symbol.len())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
            table.extend_from_slice(symbol.as_bytes());
            members.push(Member {
                series_id,
                symbol_on_day: symbol.to_owned(),
                zerodha_series_present: present(symbol),
                in_current_list: true,
            });
        }
        let slots = DaySlots::build(&members)?;
        Ok(Self {
            version: interim(AuthorityKind::Pool, DEVELOPMENT_POOL_LABEL, &table),
            members,
            slots,
        })
    }
}

/// The development pool's symbols: the F&O underlyings that are shares, in
/// `FNO_UNDERLYINGS` order.
pub fn development_symbols() -> impl Iterator<Item = &'static str> {
    brutex_core::universe::FNO_UNDERLYINGS
        .iter()
        .copied()
        .filter(|symbol| !brutex_core::universe::FNO_INDEX_UNDERLYINGS.contains(symbol))
}

impl PoolAuthority for DevelopmentPool {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn members(&self, _day: i64) -> &[Member] {
        &self.members
    }

    fn slot_of(&self, series: SeriesId, _day: i64) -> Option<u16> {
        self.slots.slot_of(series)
    }
}

// --------------------------------------------------------------- factor --

/// The shape of a stock-day's Zerodha/GDFL relation (DATA-1, PREC-13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FactorShape {
    /// One ratio holds through the day.
    Proportional,
    /// A ratio and an offset (ASHOKLEY = GDFL/2 − Rs 2.48 style).
    Affine,
    /// No one relation holds through the day.
    NonConstant,
}

/// DATA-1's per-stock-day factor pair: a Zerodha-basis price `z` converts to
/// GDFL basis as `z × gdfl_close / zerodha_close`, exactly, in `i128`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FactorPair {
    /// The Zerodha close, paisa.
    pub zerodha_close: i64,
    /// The GDFL close, paisa.
    pub gdfl_close: i64,
    /// The relation's shape.
    pub shape: FactorShape,
    /// Whether the pair was MEASURED. The interim identity is not, and an
    /// unmeasured pair proves no basis (FM-F092 (c)).
    pub measured: bool,
}

/// DATA-1's per-stock-day factor pairs.
pub trait FactorAuthority {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The stock-day's pair. O(1).
    ///
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    ///
    /// # Errors
    ///
    /// A refusal by name when no pair exists for the stock-day.
    fn factor(&self, series: SeriesId, day: i64) -> Result<FactorPair, String>;
    /// The pair at the stock-day's first matched minute, read no later than
    /// 09:16 (FM-F092 (c)). O(1).
    ///
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    ///
    /// # Errors
    ///
    /// A refusal by name when no such minute was matched.
    fn first_minute(&self, series: SeriesId, day: i64) -> Result<FactorPair, String>;
    /// Whether the authority covers `day` at all. Before its first covered
    /// day only the landing test applies, labelled.
    fn covers(&self, day: i64) -> bool;
}

/// The identity factor's label.
pub const IDENTITY_FACTOR_LABEL: &str =
    "identity factor (Zerodha basis = GDFL basis), development only, never measured";

/// The INTERIM factor: one, unmeasured, covering nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityFactor {
    version: AuthorityVersion,
}

impl IdentityFactor {
    /// The identity factor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: interim(AuthorityKind::Factor, IDENTITY_FACTOR_LABEL, &[]),
        }
    }

    /// The pair it answers.
    const IDENTITY: FactorPair = FactorPair {
        zerodha_close: 1,
        gdfl_close: 1,
        shape: FactorShape::Proportional,
        measured: false,
    };
}

impl FactorAuthority for IdentityFactor {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn factor(&self, _series: SeriesId, _day: i64) -> Result<FactorPair, String> {
        Ok(Self::IDENTITY)
    }

    fn first_minute(&self, _series: SeriesId, _day: i64) -> Result<FactorPair, String> {
        Ok(Self::IDENTITY)
    }

    fn covers(&self, _day: i64) -> bool {
        false
    }
}

// -------------------------------------------------------------- seconds --

/// DATA-2's verdict on one stock-day's GDFL seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SecondsStatus {
    /// Not checked: labelled on every deep reading.
    Unchecked,
    /// Checked and agreeing.
    Agrees,
    /// Checked and differing, by name.
    Differs(&'static str),
}

/// DATA-2's per-stock-day seconds verdicts.
pub trait SecondsTrust {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The verdict for the stock-day. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn status(&self, series: SeriesId, day: i64) -> SecondsStatus;
}

/// The unchecked-seconds label.
pub const UNCHECKED_SECONDS_LABEL: &str = "GDFL seconds unchecked (DATA-2 not built)";

/// The INTERIM seconds trust: every stock-day unchecked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncheckedSeconds {
    version: AuthorityVersion,
}

impl UncheckedSeconds {
    /// Unchecked seconds.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: interim(AuthorityKind::SecondsTrust, UNCHECKED_SECONDS_LABEL, &[]),
        }
    }
}

impl SecondsTrust for UncheckedSeconds {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn status(&self, _series: SeriesId, _day: i64) -> SecondsStatus {
        SecondsStatus::Unchecked
    }
}

// ----------------------------------------------------------- order flow --

/// OF-1's signed order-flow windows.
pub trait OrderFlowAuthority {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The window `[from_second, to_second)` of the stock-day, seconds of the
    /// IST day. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn window(
        &self,
        series: SeriesId,
        day: i64,
        from_second: u32,
        to_second: u32,
    ) -> OrderFlowWindow;
}

/// The absent-order-flow label.
pub const ABSENT_ORDER_FLOW_LABEL: &str =
    "order flow absent (OF-1 not built): the trigger_order_flow row is constant";

/// The INTERIM order flow: no source, so every window is
/// [`OrderFlowWindow::Off`] and the `trigger_order_flow` row is constant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsentOrderFlow {
    version: AuthorityVersion,
}

impl AbsentOrderFlow {
    /// Absent order flow.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: interim(AuthorityKind::OrderFlow, ABSENT_ORDER_FLOW_LABEL, &[]),
        }
    }
}

impl OrderFlowAuthority for AbsentOrderFlow {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn window(&self, _: SeriesId, _: i64, _: u32, _: u32) -> OrderFlowWindow {
        OrderFlowWindow::Off
    }
}

// ------------------------------------------------------ landing / basis --

/// One landing of stored data: one pull whose days share one adjustment basis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LandingId(pub u64);

/// A PREC-10 basis-break record: a sourced ratio that carries a close from
/// the old basis to the new one, applied exactly in `i128`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BasisBreak {
    /// The ratio's numerator.
    pub numerator: i64,
    /// The ratio's denominator, never zero.
    pub denominator: i64,
    /// Where the ratio comes from.
    pub source: &'static str,
}

/// Landings and basis breaks.
pub trait LandingAuthority {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The landing the stock-day's stored minutes came from. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn landing(&self, series: SeriesId, day: i64) -> Option<LandingId>;
    /// A PREC-10 basis-break record between `from_day` and `day`. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn basis_break(&self, series: SeriesId, from_day: i64, day: i64) -> Option<BasisBreak>;
}

/// The single-landing label.
pub const SINGLE_LANDING_LABEL: &str = "single landing assumed (no landing ledger, no PREC-10 \
     record): Zerodha's back-adjusted history read as one basis, development only";

/// The INTERIM landing: every stored day is one landing, and no break is
/// recorded. Refused for `--out-store`: one basis across the history is an
/// assumption, not a proof (FM-F092).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleLanding {
    version: AuthorityVersion,
}

impl SingleLanding {
    /// The single-landing assumption.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: interim(AuthorityKind::Landing, SINGLE_LANDING_LABEL, &[]),
        }
    }
}

impl LandingAuthority for SingleLanding {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn landing(&self, _series: SeriesId, _day: i64) -> Option<LandingId> {
        Some(LandingId(0))
    }

    fn basis_break(&self, _: SeriesId, _: i64, _: i64) -> Option<BasisBreak> {
        None
    }
}

/// How a basis was proven (FM-F092 (c)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BasisProof {
    /// Both days are from one landing.
    SameLanding,
    /// A PREC-10 record carries the close across the break, exactly.
    BreakRecord(BasisBreak),
    /// DATA-1's pair for D-1 equals the pair at D's first matched minute.
    FactorPairsEqual,
}

/// Why a basis was not proven.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BasisRefusal {
    /// No proof holds: `basis_unproven`.
    Unproven,
    /// DATA-1 or PREC-13 marks the day non-constant or non-proportional:
    /// `basis_unreliable`.
    Unreliable,
    /// The forward path: no NSE corporate-action list was read for the
    /// stock-day (FIB-15 4.8).
    UnprovenForward,
    /// The forward path: a measured step with no NSE row.
    Unsourced,
    /// The forward path: the re-read previous session was revised or differs.
    Differs,
    /// The forward path: an NSE row names the day and the stock is not
    /// rebuilt on the new basis yet.
    RebuildPending,
    /// The forward path: a latch set on an earlier refusal holds until a
    /// sealed rebuild clears it.
    Latched {
        /// The day the latch was set.
        since: i64,
    },
    /// The forward path: no NSE corporate-action source exists at all.
    SourceMissing,
    /// The forward path: the source exists and the day's list is not read
    /// yet: PENDING, pool-wide.
    Pending,
}

impl BasisRefusal {
    /// The counted name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unproven | Self::UnprovenForward => "basis_unproven",
            Self::Unreliable => "basis_unreliable",
            Self::Unsourced => "basis_unsourced",
            Self::Differs => "basis_differs",
            Self::RebuildPending => "basis_rebuild_pending",
            Self::Latched { .. } => "basis_unresolved",
            Self::SourceMissing => "basis_source_missing",
            Self::Pending => "corporate_actions_unread",
        }
    }
}

/// The basis verdict for carrying a previous close into a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BasisVerdict {
    /// The proof, or the refusal.
    pub outcome: Result<BasisProof, BasisRefusal>,
    /// Whether only the landing test could apply, because the factor
    /// authority does not cover the day: labelled `landing_only`.
    pub landing_only: bool,
}

/// The per-stock-day basis verdict (RQ-4), consumed through this trait in the
/// historical and the forward path alike.
pub trait BasisVerdictAuthority {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The verdict for carrying `series`'s close of `previous_day` into `day`.
    /// O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn verdict(&self, series: SeriesId, previous_day: i64, day: i64) -> BasisVerdict;
}

/// The historical basis verdict: FM-F092's proof over a landing authority and
/// a factor authority.
///
/// In order: a day DATA-1 marks affine or non-constant is `basis_unreliable`;
/// both days from one landing is [`BasisProof::SameLanding`]; a PREC-10 record
/// is [`BasisProof::BreakRecord`]; DATA-1's MEASURED pair for D-1 equal (as a
/// ratio, cross-multiplied in `i128`) to its pair at D's first matched minute
/// is [`BasisProof::FactorPairsEqual`]; anything else is `basis_unproven`. An
/// unmeasured pair (the interim identity) proves nothing, so a raw-to-raw
/// close across an ex-date is never accepted on a factor of one (FIB-15 4.8).
pub struct HistoricalBasis<'a> {
    version: AuthorityVersion,
    landings: &'a dyn LandingAuthority,
    factors: &'a dyn FactorAuthority,
}

/// The historical verdict's label.
pub const HISTORICAL_BASIS_LABEL: &str =
    "FM-F092 historical basis proof: same landing, a PREC-10 record, or DATA-1's pairs equal";

impl<'a> HistoricalBasis<'a> {
    /// The verdict over `landings` and `factors`. It is interim exactly when
    /// either input is.
    #[must_use]
    pub fn new(landings: &'a dyn LandingAuthority, factors: &'a dyn FactorAuthority) -> Self {
        let mut table = Vec::new();
        table.extend_from_slice(&landings.version().digest);
        table.extend_from_slice(&factors.version().digest);
        let kind = AuthorityKind::Basis;
        let word = kind.interim_word();
        Self {
            version: AuthorityVersion {
                kind,
                word,
                label: HISTORICAL_BASIS_LABEL,
                digest: version_digest(kind, word, HISTORICAL_BASIS_LABEL, &table),
                interim: landings.version().interim || factors.version().interim,
            },
            landings,
            factors,
        }
    }
}

/// Whether two measured pairs state one ratio, cross-multiplied in `i128`.
fn same_ratio(a: FactorPair, b: FactorPair) -> bool {
    a.measured
        && b.measured
        && i128::from(a.gdfl_close) * i128::from(b.zerodha_close)
            == i128::from(b.gdfl_close) * i128::from(a.zerodha_close)
}

impl BasisVerdictAuthority for HistoricalBasis<'_> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn verdict(&self, series: SeriesId, previous_day: i64, day: i64) -> BasisVerdict {
        let landing_only = !self.factors.covers(day);
        let verdict = |outcome| BasisVerdict {
            outcome,
            landing_only,
        };
        if !landing_only
            && let Ok(pair) = self.factors.factor(series, day)
            && pair.measured
            && pair.shape != FactorShape::Proportional
        {
            return verdict(Err(BasisRefusal::Unreliable));
        }
        let today = self.landings.landing(series, day);
        if today.is_some() && today == self.landings.landing(series, previous_day) {
            return verdict(Ok(BasisProof::SameLanding));
        }
        if landing_only {
            return verdict(Err(BasisRefusal::Unproven));
        }
        if let Some(record) = self.landings.basis_break(series, previous_day, day)
            && record.denominator != 0
        {
            return verdict(Ok(BasisProof::BreakRecord(record)));
        }
        match (
            self.factors.factor(series, previous_day),
            self.factors.first_minute(series, day),
        ) {
            (Ok(before), Ok(first)) if same_ratio(before, first) => {
                verdict(Ok(BasisProof::FactorPairsEqual))
            }
            _ => verdict(Err(BasisRefusal::Unproven)),
        }
    }
}

/// The class of a forward day's re-read previous session against the stored
/// one (FIB-15 4.8): same | ratio_step | affine_step | revised | differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReReadClass {
    /// Byte-equal bars.
    Same,
    /// Every bar moved by one ratio.
    RatioStep,
    /// Every bar moved by one ratio and offset.
    AffineStep,
    /// Some bars revised.
    Revised,
    /// No relation.
    Differs,
}

/// What the ingest stage (RQ-16) read for one forward stock-day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CorporateActions {
    /// No NSE corporate-action source exists at all.
    NoSource,
    /// The source exists and the day's list is not read yet.
    Unread,
    /// The day's list was read for the stock.
    Read {
        /// Whether it holds a row for the stock with ex-date on the day.
        row_for_day: bool,
        /// Whether the stock has been rebuilt on the new basis since.
        rebuilt: bool,
        /// The re-read class of the previous session, or `None` when the
        /// stock was not re-read.
        re_read: Option<ReReadClass>,
    },
}

/// The forward basis rule of FIB-15 4.8 with its latch: a measured class may
/// only REFUSE; a stock is accepted on D only when the NSE list for D was read
/// and either it has no row for the stock with ex-date D and the re-read class
/// is `same`, or it has a row and the stock was rebuilt. Any refusal latches
/// `basis_unresolved since D` until a sealed rebuild clears it; a later `same`
/// re-read never does. One stock's state, O(1) per day.
/// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ForwardBasis {
    latched_since: Option<i64>,
}

impl ForwardBasis {
    /// A stock with no latch.
    #[must_use]
    pub const fn unlatched() -> Self {
        Self {
            latched_since: None,
        }
    }

    /// The day the latch was set, if it is set.
    #[must_use]
    pub const fn latched_since(&self) -> Option<i64> {
        self.latched_since
    }

    /// The verdict for `day` given what the ingest stage read, updating the
    /// latch. A sealed rebuild (`rebuilt` in the read) clears an earlier
    /// latch before the day is judged.
    pub fn judge(&mut self, day: i64, read: CorporateActions) -> Result<(), BasisRefusal> {
        // No source, or the day's list unread: refused or pending pool-wide,
        // which is no fact about this stock, so neither sets its latch.
        let (row_for_day, rebuilt, re_read) = match read {
            CorporateActions::NoSource => return Err(BasisRefusal::SourceMissing),
            CorporateActions::Unread => return Err(BasisRefusal::Pending),
            CorporateActions::Read {
                row_for_day,
                rebuilt,
                re_read,
            } => (row_for_day, rebuilt, re_read),
        };
        if rebuilt {
            self.latched_since = None;
        }
        if let Some(since) = self.latched_since {
            return Err(BasisRefusal::Latched { since });
        }
        let outcome = match (row_for_day, rebuilt, re_read) {
            (true, true, _) | (false, _, Some(ReReadClass::Same)) => Ok(()),
            (_, _, Some(ReReadClass::Revised | ReReadClass::Differs)) => Err(BasisRefusal::Differs),
            (false, _, Some(ReReadClass::RatioStep | ReReadClass::AffineStep)) => {
                Err(BasisRefusal::Unsourced)
            }
            (false, _, None) => Err(BasisRefusal::UnprovenForward),
            (true, false, _) => Err(BasisRefusal::RebuildPending),
        };
        if outcome.is_err() {
            self.latched_since = Some(day);
        }
        outcome
    }
}

// ----------------------------------------------------------------- tail --

/// What a witness says about the time after a day's last Zerodha minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TailReading {
    /// Trades happened after it: the day was cut (`tail_cut`).
    TradesAfter,
    /// No trade happened after it: kept, labelled `no_trades_after`.
    NoTradesAfter,
    /// No witness: `tail_unwitnessed`, labelled.
    Unknown,
}

/// DATA-1's or the D-3335 gap ledger's witness of a day's tail (FM-F094).
pub trait TailWitness {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// Whether trades happened on the stock-day after `last_second` (a second
    /// of the IST day). O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn after(&self, series: SeriesId, day: i64, last_second: u32) -> TailReading;
}

/// The unwitnessed label.
pub const UNWITNESSED_TAIL_LABEL: &str =
    "no tail witness (DATA-1 and the D-3335 gap ledger not read): a short day is tail_unwitnessed";

/// The INTERIM tail witness: no witness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnwitnessedTail {
    version: AuthorityVersion,
}

impl UnwitnessedTail {
    /// No witness.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: interim(AuthorityKind::TailWitness, UNWITNESSED_TAIL_LABEL, &[]),
        }
    }
}

impl TailWitness for UnwitnessedTail {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn after(&self, _: SeriesId, _: i64, _: u32) -> TailReading {
        TailReading::Unknown
    }
}

// --------------------------------------------------------------- screen --

/// PREC-9's verdict on one index day (FM-F087). No own-past magnitude test
/// ever produces one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexDay {
    /// PREC-9 is not merged: stamped as measured, labelled `index day unscreened`.
    Unscreened,
    /// Screened and clean.
    Clean,
    /// A foreign index's day.
    Foreign,
    /// A rescaled day.
    Rescaled,
    /// A junk day.
    Junk,
}

impl IndexDay {
    /// The reason a stamp is absent on this day, or `None` when it is stamped.
    #[must_use]
    pub const fn absent_reason(self) -> Option<&'static str> {
        match self {
            Self::Unscreened | Self::Clean => None,
            Self::Foreign => Some("index_day_foreign"),
            Self::Rescaled => Some("index_day_rescaled"),
            Self::Junk => Some("index_day_junk"),
        }
    }
}

/// PREC-9's index-day census.
pub trait IndexScreen {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The verdict for `index` on `day`. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn screen(&self, index: &str, day: i64) -> IndexDay;
}

/// The unscreened label.
pub const UNSCREENED_LABEL: &str = "index day unscreened (PREC-9 not merged)";

/// The INTERIM screen: every index day unscreened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnscreenedIndices {
    version: AuthorityVersion,
}

impl UnscreenedIndices {
    /// Every day unscreened.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: interim(AuthorityKind::IndexScreen, UNSCREENED_LABEL, &[]),
        }
    }
}

impl IndexScreen for UnscreenedIndices {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn screen(&self, _: &str, _: i64) -> IndexDay {
        IndexDay::Unscreened
    }
}

// --------------------------------------------------------------- expiry --

/// The expiry stamps of one stock-day (FM-F098): reference only, never a
/// coordinate and never an identity term.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpiryStamp {
    /// The day is neither the stock's monthly expiry nor an index weekly one.
    None,
    /// The stock's monthly expiry.
    StockMonthly,
    /// An index weekly expiry.
    IndexWeekly,
    /// Both.
    Both,
    /// No expiry source answered, by name.
    Absent(&'static str),
}

/// The F&O ban stamp: always absent, the ban list is UNVERIFIED.
pub const FNO_BAN_ABSENT: &str = "ban list UNVERIFIED";

/// Expiry days from stored contract expiries or the settled-expiry authority,
/// never from `costs::expiry`.
pub trait ExpiryAuthority {
    /// The implementation's version.
    fn version(&self) -> &AuthorityVersion;
    /// The stamp for `series` on `day`. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    fn expiry(&self, series: SeriesId, day: i64) -> ExpiryStamp;
}

/// The unloaded-expiry label.
pub const UNLOADED_EXPIRY_LABEL: &str =
    "no expiry source loaded (stored contract expiries not read): expiry stamps absent";

/// The INTERIM expiry authority: every stamp absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnloadedExpiry {
    version: AuthorityVersion,
}

impl UnloadedExpiry {
    /// Every stamp absent.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: interim(AuthorityKind::Expiry, UNLOADED_EXPIRY_LABEL, &[]),
        }
    }
}

impl ExpiryAuthority for UnloadedExpiry {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn expiry(&self, _: SeriesId, _: i64) -> ExpiryStamp {
        ExpiryStamp::Absent("expiry_source_not_loaded")
    }
}

// ------------------------------------------------------------- registry --

/// What a flag word selected for one kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Choice {
    /// The kind's interim implementation.
    Interim,
}

/// Maps one flag word to an implementation of `kind`.
///
/// # Errors
///
/// A new-work word is routed to [`super::wire::register`] and its refusal
/// returned (every one until FIB-10); a word that is neither the interim word
/// nor the kind's new-work word is refused naming both.
pub fn select(kind: AuthorityKind, word: &str) -> Result<Choice, String> {
    if word == kind.interim_word() {
        return Ok(Choice::Interim);
    }
    if Some(word) == kind.new_work_word() {
        // Routed to FIB-10's register, whose refusal is the answer until it
        // is built; a word it accepts still has no implementation here.
        return wire::register(word).and(Err(format!(
            "authority `{word}` is registered by FIB-10 but not wired into FIB-8's registry"
        )));
    }
    Err(format!(
        "`{word}` names no {} authority; the words are {}{}",
        kind.name(),
        kind.interim_word(),
        kind.new_work_word()
            .map_or_else(String::new, |new| format!(" and {new} (FIB-10)"))
    ))
}

/// Every authority a run reads, each selected by name.
pub struct Authorities {
    /// The calendar.
    pub calendar: Box<dyn CalendarAuthority>,
    /// The pool.
    pub pool: Box<dyn PoolAuthority>,
    /// The factor pairs.
    pub factor: Box<dyn FactorAuthority>,
    /// The seconds verdicts.
    pub seconds: Box<dyn SecondsTrust>,
    /// The order flow.
    pub order_flow: Box<dyn OrderFlowAuthority>,
    /// The landings.
    pub landing: Box<dyn LandingAuthority>,
    /// The tail witness.
    pub tail: Box<dyn TailWitness>,
    /// The index screen.
    pub screen: Box<dyn IndexScreen>,
    /// The expiry stamps.
    pub expiry: Box<dyn ExpiryAuthority>,
}

/// The flags that select every authority: `--<kind> <word>` per kind, or
/// `--authorities interim` for every interim one at once. No authority is
/// ever chosen for a caller who named none.
///
/// # Errors
///
/// `authorities_required` naming every flag when none is given or a kind is
/// left unnamed; every [`select`] refusal.
pub fn words_of(arguments: &[&str]) -> Result<Vec<(AuthorityKind, String)>, String> {
    let value = |flag: &str| {
        arguments
            .iter()
            .position(|word| *word == flag)
            .and_then(|at| arguments.get(at + 1))
            .copied()
    };
    let all = value("--authorities");
    let mut chosen = Vec::new();
    let mut unnamed = Vec::new();
    for &kind in AuthorityKind::ALL {
        let flag = format!("--{}", kind.name());
        let word = match (value(&flag), all) {
            (Some(word), _) => word,
            (None, Some("interim")) => kind.interim_word(),
            (None, _) => {
                unnamed.push(format!("{flag} {}", kind.interim_word()));
                continue;
            }
        };
        select(kind, word)?;
        chosen.push((kind, word.to_owned()));
    }
    if let Some(other) = all.filter(|word| *word != "interim") {
        return Err(format!(
            "`--authorities {other}` is not a set; the only set is `--authorities interim`"
        ));
    }
    if !unnamed.is_empty() {
        return Err(format!(
            "authorities_required: every authority is named, never defaulted; name {} or \
             give `--authorities interim`",
            unnamed.join(", ")
        ));
    }
    Ok(chosen)
}

impl Authorities {
    /// Every interim authority, the pool's series presence answered by
    /// `present`.
    ///
    /// # Errors
    ///
    /// Every [`DevelopmentPool::new`] refusal.
    pub fn interim(present: &dyn Fn(&str) -> bool) -> Result<Self, String> {
        Ok(Self {
            calendar: Box::new(InterimCalendar::new()),
            pool: Box::new(DevelopmentPool::new(present)?),
            factor: Box::new(IdentityFactor::new()),
            seconds: Box::new(UncheckedSeconds::new()),
            order_flow: Box::new(AbsentOrderFlow::new()),
            landing: Box::new(SingleLanding::new()),
            tail: Box::new(UnwitnessedTail::new()),
            screen: Box::new(UnscreenedIndices::new()),
            expiry: Box::new(UnloadedExpiry::new()),
        })
    }

    /// The historical basis verdict over this set's landings and factors.
    #[must_use]
    pub fn basis(&self) -> HistoricalBasis<'_> {
        HistoricalBasis::new(self.landing.as_ref(), self.factor.as_ref())
    }

    /// Every version, in [`AuthorityKind::ALL`] order with the basis verdict
    /// in its place: the run's authority identity leaves.
    #[must_use]
    pub fn versions(&self) -> Vec<AuthorityVersion> {
        vec![
            self.calendar.version().clone(),
            self.pool.version().clone(),
            self.factor.version().clone(),
            self.seconds.version().clone(),
            self.order_flow.version().clone(),
            self.landing.version().clone(),
            self.basis().version().clone(),
            self.tail.version().clone(),
            self.screen.version().clone(),
            self.expiry.version().clone(),
        ]
    }

    /// Refuses every interim authority for `--out-store`, by name.
    ///
    /// # Errors
    ///
    /// `interim_authority_for_out_store`, naming each interim kind and its
    /// label.
    pub fn refuse_for_out_store(&self) -> Result<(), String> {
        let interim: Vec<String> = self
            .versions()
            .into_iter()
            .filter(|version| version.interim)
            .map(|version| {
                format!(
                    "{} = {} ({})",
                    version.kind.name(),
                    version.word,
                    version.label
                )
            })
            .collect();
        if interim.is_empty() {
            return Ok(());
        }
        Err(format!(
            "interim_authority_for_out_store: --out-store never rests on a stand-in; interim: {}",
            interim.join("; ")
        ))
    }
}

#[cfg(test)]
#[path = "authorities_tests.rs"]
mod tests;
