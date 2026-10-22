//! The share corporate-action table and the measured basis runs (R4-26:
//! COR-06, Z8, FM-260), both store tables read through R3-30's ONE loader
//! ([`crate::reference_data::load_one`]).
//!
//! # Why
//!
//! Zerodha back-adjusts a share's prices (and volume) for splits and bonuses;
//! GDFL does not. On every day of a run between two actions the ratio of the
//! two feeds' closes is one constant (HDFCBANK sits at Zerodha = GDFL x 0.25
//! for years, then 0.5, then 1), and at an ex-date it STEPS. Nothing kept
//! those runs or those steps: the engine said "corporate actions unchecked"
//! (D-0694) while the chain held the evidence, and a 2:1 split is the largest
//! "winner" a sweep over raw GDFL prices can find (FM-260).
//!
//! Two tables close that, each a file in the store with a receipt (source,
//! the sha256 of the source bytes, the day it was read), a checksum, and a
//! refusal BY NAME when the bytes are not the bytes written:
//!
//! - `basis_runs` ([`BasisRuns`]): per share, the maximal runs of one
//!   Zerodha/GDFL ratio, folded by `cli basis-runs` from DATA-1's per
//!   stock-day factor (`cli::feed_reconcile::FactorIndex`, the ONE owner of
//!   that factor: this module never measures a ratio of its own). Always
//!   `measured`, never `sourced`.
//! - `corporate_actions` ([`CorporateActions`]): per share, its ex-dates with
//!   kind and ratio, each row tagged `nse` (read from an NSE file the charter
//!   records) or `measured` (one step between two basis runs).
//!
//! # The law: a measured row LABELS and never EXCUSES (`CLAUDE.md` §3 rule 1)
//!
//! A measured row is the data describing itself. It may put a note on a day
//! ([`CorporateActions::labels`]), but it never suppresses a refusal
//! ([`CorporateActions::excuse`] returns only `nse` rows) and never decides a
//! class ([`Action::class`] is `None` for it, and its kind is always
//! `unclassed`: a ratio step cannot tell a split from a bonus). Only a row
//! tagged `nse`, in a file whose standing is `sourced`, does either; the
//! loader refuses an `nse` row in a measured file and a measured row that
//! names a kind (`a_measured_row_never_suppresses_a_refusal_or_decides_a_class`).
//! The NSE corporate-actions file is UNVERIFIED as fetchable unattended (its
//! endpoint is cookie-gated); until one is recorded in `docs/00-charter.md`
//! every row a store holds is measured.
//!
//! # Exact, never a float
//!
//! Every ratio is a reduced fraction of integers ([`Ratio`]), every
//! comparison a cross-multiplication. Each day's interval is DATA-1's own
//! allowance interval; a run's band is the intersection of its days'
//! intervals each widened by [`STEP_PERMILLE`] (D-3339's `basis_step`
//! threshold), and its ratio the SIMPLEST fraction in that band, so a 2:1
//! split reads `1/2` then `1/1`. A step's factor is the simplest fraction
//! inside `[new.low / old.high, new.high / old.low]` of the two bands: the
//! tolerance is the measurement's interval and D-3339's threshold, not a
//! number chosen here.
//!
//! # Cost
//!
//! A load is one read of the newest generation and one build, O(rows). Every
//! lookup after it is one hash of the symbol to that symbol's sorted slice
//! and one binary search of it: at most [`MAX_ACTIONS_PER_SYMBOL`] rows
//! (seven comparisons) or [`MAX_RUNS_PER_SYMBOL`] runs (twelve), a table
//! holding more for one symbol is refused by name at load. The fold
//! ([`fold_runs`]) is O(days) per share, one constant step a day; the
//! checker ([`check`]) is O(runs + rows). Named for `docs/06-limits.md`.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use crate::reference_data::{Draft, Parsed, Refusal, RefusalKind, Standing, Table, parse_day};
use crate::session::Day;

/// The most actions one symbol may hold, so a lookup is one hash and at most
/// seven comparisons. A table holding more is refused by name.
pub const MAX_ACTIONS_PER_SYMBOL: usize = 64;

/// The most basis runs one symbol may hold, so a lookup is one hash and at
/// most twelve comparisons. A table holding more is refused by name.
pub const MAX_RUNS_PER_SYMBOL: usize = 4_096;

/// The largest numerator or denominator a [`Ratio`] holds: two of them
/// multiply inside a `u128` with room to spare (`10^36 < 3.4 x 10^38`).
pub const MAX_TERM: u128 = 1_000_000_000_000_000_000;

/// The decision that owns both tables.
pub const DECISION: &str = "D-6892";

/// D-3339 item 3's threshold, the one `/sources` judges `basis_step` by
/// (`api::sources::FACTOR_THRESHOLD_PERMILLE`, pinned equal by a test): a
/// ratio that moves by more than this many permille between consecutive
/// days with a basis STEPS, and a run's days lie within this many permille
/// of one ratio. Chosen, not sourced: the threshold that reproduces D-3323's
/// 101 of 208 shares. Without it every quantisation wobble of an adjusted
/// series (measured: a move of 0.03% between two days whose intervals are
/// disjoint) would be a "corporate action".
pub const STEP_PERMILLE: u16 = 2;

// ─────────────────────────────── the fraction ───────────────────────────────

/// An exact positive fraction, reduced, each term at most [`MAX_TERM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ratio {
    num: u128,
    den: u128,
}

const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

impl Ratio {
    /// `1/1`.
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// `num / den`, reduced; `None` when either is zero or a reduced term
    /// is larger than [`MAX_TERM`].
    #[must_use]
    pub const fn new(num: u128, den: u128) -> Option<Self> {
        if num == 0 || den == 0 {
            return None;
        }
        let g = gcd(num, den);
        let (num, den) = (num / g, den / g);
        if num > MAX_TERM || den > MAX_TERM {
            return None;
        }
        Some(Self { num, den })
    }

    /// From DATA-1's signed fraction; `None` unless it is positive and fits.
    #[must_use]
    pub fn of_signed(num: i128, den: i128) -> Option<Self> {
        let (num, den) = match (u128::try_from(num), u128::try_from(den)) {
            (Ok(num), Ok(den)) => (num, den),
            _ => (u128::try_from(num.checked_neg()?).ok()?, u128::try_from(den.checked_neg()?).ok()?),
        };
        Self::new(num, den)
    }

    /// The numerator.
    #[must_use]
    pub const fn num(self) -> u128 {
        self.num
    }

    /// The denominator.
    #[must_use]
    pub const fn den(self) -> u128 {
        self.den
    }

    /// `a/b` or a bare integer `a`, strictly digits; `None` for anything
    /// else, zero, or a term over [`MAX_TERM`].
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let digits = |part: &str| -> Option<u128> {
            if part.is_empty() || part.len() > 19 || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        };
        match text.split_once('/') {
            Some((num, den)) => Self::new(digits(num)?, digits(den)?),
            None => Self::new(digits(text)?, 1),
        }
    }

    /// `self x other`, reduced; `None` when a term would pass [`MAX_TERM`].
    #[must_use]
    pub fn times(self, other: Self) -> Option<Self> {
        let g1 = gcd(self.num, other.den);
        let g2 = gcd(other.num, self.den);
        let num = (self.num / g1).checked_mul(other.num / g2)?;
        let den = (self.den / g2).checked_mul(other.den / g1)?;
        Self::new(num, den)
    }

    /// `self / other`, reduced; `None` when a term would pass [`MAX_TERM`].
    #[must_use]
    pub fn over(self, other: Self) -> Option<Self> {
        self.times(Self {
            num: other.den,
            den: other.num,
        })
    }

    /// The fraction with the smallest denominator (then numerator) inside
    /// `[low, high]`, both ends included; `None` when `low > high`. Exact: a
    /// continued-fraction walk, at most as many steps as Euclid's on the
    /// larger term.
    #[must_use]
    pub fn simplest_between(low: Self, high: Self) -> Option<Self> {
        if low > high {
            return None;
        }
        let (num, den) = simplest(low.num, low.den, high.num, high.den)?;
        Self::new(num, den)
    }

    /// `level x den / num`: a Zerodha-basis level on GDFL's raw basis under
    /// this Zerodha/GDFL ratio, exactly, as a fraction (the caller rounds at
    /// its own grid). `None` for a level that is not positive or too large.
    #[must_use]
    pub fn to_gdfl(self, zerodha_paisa: i64) -> Option<Self> {
        let level = u128::try_from(zerodha_paisa).ok()?;
        Self::new(level, 1)?.over(self)
    }
}

/// The simplest fraction in `[a/b, c/d]`, `a/b <= c/d`, all positive.
fn simplest(a: u128, b: u128, c: u128, d: u128) -> Option<(u128, u128)> {
    let q = a.checked_div(b)?;
    let r = a.checked_rem(b)?;
    if r == 0 {
        return Some((q, 1));
    }
    let next = q.checked_add(1)?;
    if next.checked_mul(d)? <= c {
        return Some((next, 1));
    }
    // Both ends lie in (q, q + 1): x = q + 1 / y, y in [d / (c - qd), b / r].
    let c_less = c.checked_sub(q.checked_mul(d)?)?;
    let (p, s) = simplest(d, c_less, b, r)?;
    Some((q.checked_mul(p)?.checked_add(s)?, p))
}

impl Ord for Ratio {
    fn cmp(&self, other: &Self) -> Ordering {
        // Terms are at most 10^18, so each product is at most 10^36.
        let left = self.num.saturating_mul(other.den);
        let right = other.num.saturating_mul(self.den);
        left.cmp(&right)
    }
}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

// ─────────────────────────────── the actions ───────────────────────────────

/// What an action is. A measured row is always [`Kind::Unclassed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// A face-value split.
    Split,
    /// A bonus issue.
    Bonus,
    /// A rights issue.
    Rights,
    /// A demerger.
    Demerger,
    /// A merger.
    Merger,
    /// A dividend.
    Dividend,
    /// A ratio step measured from the data: the data cannot say which kind.
    Unclassed,
}

impl Kind {
    /// Every kind, in wire order.
    pub const ALL: [Self; 7] = [
        Self::Split,
        Self::Bonus,
        Self::Rights,
        Self::Demerger,
        Self::Merger,
        Self::Dividend,
        Self::Unclassed,
    ];

    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Bonus => "bonus",
            Self::Rights => "rights",
            Self::Demerger => "demerger",
            Self::Merger => "merger",
            Self::Dividend => "dividend",
            Self::Unclassed => "unclassed",
        }
    }

    /// The kind a word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }
}

/// Where one row came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    /// Read from an NSE file the charter records: may excuse and class.
    Nse,
    /// One step between two measured basis runs: labels only.
    Measured,
}

impl Tag {
    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Nse => "nse",
            Self::Measured => "measured",
        }
    }

    /// The tag a word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        [Self::Nse, Self::Measured]
            .into_iter()
            .find(|tag| tag.word() == word)
    }
}

/// One row of the corporate-action table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// The share, as the store files it.
    pub symbol: String,
    /// Its ISIN, when the row carries one.
    pub isin: Option<String>,
    /// The ex-date. On a day with no session the action applies at the next
    /// session: a reader asks for the rows in `(previous session, session]`.
    pub ex_date: Day,
    /// The kind; [`Kind::Unclassed`] for every measured row.
    pub kind: Kind,
    /// The price factor: one old share becomes `num / den` new ones, so a
    /// Zerodha/GDFL ratio steps by exactly this much across the ex-date.
    pub ratio: Ratio,
    /// Where the row came from.
    pub tag: Tag,
    /// For a measured row, the last day seen on the old basis: the ex-date
    /// lies in `(after, ex_date]`. `None` for an NSE row.
    pub after: Option<Day>,
    /// The circular, file or basis-run step the row rests on.
    pub evidence: String,
}

impl Action {
    /// Whether this row may suppress a refusal: only an `nse` row.
    #[must_use]
    pub const fn excuses(&self) -> bool {
        matches!(self.tag, Tag::Nse)
    }

    /// The class this row decides, `None` for a measured row: a measured
    /// row labels and never decides a class.
    #[must_use]
    pub const fn class(&self) -> Option<Kind> {
        match self.tag {
            Tag::Nse => Some(self.kind),
            Tag::Measured => None,
        }
    }

    /// The row as the table's fields, in column order.
    #[must_use]
    pub fn fields(&self) -> Vec<String> {
        vec![
            self.symbol.clone(),
            self.isin.clone().unwrap_or_else(|| "-".to_owned()),
            self.ex_date.to_string(),
            self.kind.word().to_owned(),
            self.ratio.num().to_string(),
            self.ratio.den().to_string(),
            self.tag.word().to_owned(),
            self.after.map_or_else(|| "-".to_owned(), |day| day.to_string()),
            self.evidence.clone(),
        ]
    }
}

/// The corporate-action table, loaded: per symbol its rows sorted by
/// ex-date, behind one hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorporateActions {
    standing: Standing,
    rows: usize,
    by_symbol: HashMap<String, Vec<Action>>,
}

/// A refusal of a row of `table` at `line` in the file at `path`.
fn refusal(
    table: Table,
    parsed: &Parsed,
    path: &Path,
    kind: RefusalKind,
    line: usize,
    detail: String,
) -> Refusal {
    Refusal {
        table,
        path: path.to_path_buf(),
        generation: Some(parsed.receipt.generation),
        kind,
        detail: format!("line {line}: {detail}"),
    }
}

fn day_or_dash(text: &str) -> Result<Option<Day>, String> {
    if text == "-" {
        return Ok(None);
    }
    parse_day(text)
        .map(Some)
        .ok_or_else(|| format!("{text:?} is not a YYYY-MM-DD day"))
}

fn isin_ok(text: &str) -> bool {
    text.len() == 12
        && text
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

impl CorporateActions {
    /// Builds the table from a parsed file: the loader's `build` for
    /// [`Table::CorporateActions`].
    ///
    /// # Errors
    ///
    /// A malformed row, an `nse` row in a file that is not `sourced`, a
    /// measured row that names a kind, an `nse` row that does not, two rows
    /// of one (symbol, ex-date, kind), or more than
    /// [`MAX_ACTIONS_PER_SYMBOL`] rows for one symbol; each by name.
    pub fn build(parsed: &Parsed, path: &Path) -> Result<Self, Refusal> {
        let table = Table::CorporateActions;
        let standing = parsed.receipt.standing;
        let mut by_symbol: HashMap<String, Vec<Action>> = HashMap::new();
        for (line, row) in &parsed.rows {
            let bad = |kind, detail: String| refusal(table, parsed, path, kind, *line, detail);
            let [symbol, isin, ex_date, kind, num, den, tag, after, evidence] = row.as_slice()
            else {
                return Err(bad(RefusalKind::Row, "wrong field count".to_owned()));
            };
            let action = Action {
                symbol: symbol.clone(),
                isin: match isin.as_str() {
                    "-" => None,
                    text if isin_ok(text) => Some(text.to_owned()),
                    text => {
                        return Err(bad(
                            RefusalKind::Row,
                            format!("{text:?} is not a 12-character ISIN"),
                        ));
                    }
                },
                ex_date: day_or_dash(ex_date)
                    .map_err(|why| bad(RefusalKind::Row, why))?
                    .ok_or_else(|| bad(RefusalKind::Row, "ex_date is required".to_owned()))?,
                kind: Kind::parse(kind)
                    .ok_or_else(|| bad(RefusalKind::Row, format!("{kind:?} is not a kind")))?,
                ratio: num
                    .parse::<u128>()
                    .ok()
                    .zip(den.parse::<u128>().ok())
                    .and_then(|(num, den)| Ratio::new(num, den))
                    .ok_or_else(|| {
                        bad(
                            RefusalKind::Row,
                            format!("{num}/{den} is not a positive ratio of terms up to 10^18"),
                        )
                    })?,
                tag: Tag::parse(tag)
                    .ok_or_else(|| bad(RefusalKind::Row, format!("{tag:?} is neither nse nor measured")))?,
                after: day_or_dash(after).map_err(|why| bad(RefusalKind::Row, why))?,
                evidence: evidence.clone(),
            };
            match action.tag {
                Tag::Nse if standing != Standing::Sourced => {
                    return Err(bad(
                        RefusalKind::Row,
                        format!(
                            "an nse row in a {} file: only a sourced file may carry one",
                            standing.word()
                        ),
                    ));
                }
                Tag::Nse if action.kind == Kind::Unclassed => {
                    return Err(bad(
                        RefusalKind::Row,
                        "an nse row must name its kind".to_owned(),
                    ));
                }
                Tag::Measured if action.kind != Kind::Unclassed => {
                    return Err(bad(
                        RefusalKind::Row,
                        format!(
                            "a measured row names the kind {}: a measured row never decides a class",
                            action.kind.word()
                        ),
                    ));
                }
                _ => {}
            }
            if action.after.is_some_and(|after| after >= action.ex_date) {
                return Err(bad(
                    RefusalKind::Row,
                    format!("after {after:?} is not before the ex-date", after = action.after),
                ));
            }
            let held = by_symbol.entry(symbol.clone()).or_default();
            if held
                .iter()
                .any(|other| other.ex_date == action.ex_date && other.kind == action.kind)
            {
                return Err(bad(
                    RefusalKind::Duplicate,
                    format!(
                        "{symbol:?} has two {} rows on {}",
                        action.kind.word(),
                        action.ex_date
                    ),
                ));
            }
            if held.len() >= MAX_ACTIONS_PER_SYMBOL {
                return Err(bad(
                    RefusalKind::TooMany,
                    format!("{symbol:?} has more than {MAX_ACTIONS_PER_SYMBOL} rows"),
                ));
            }
            held.push(action);
        }
        for rows in by_symbol.values_mut() {
            rows.sort_by(|a, b| a.ex_date.cmp(&b.ex_date).then(a.kind.cmp(&b.kind)));
        }
        Ok(Self {
            standing,
            rows: parsed.rows.len(),
            by_symbol,
        })
    }

    /// What the file rests on.
    #[must_use]
    pub const fn standing(&self) -> Standing {
        self.standing
    }

    /// How many rows the table holds.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Every row of `symbol`, by ex-date: one hash.
    #[must_use]
    pub fn of(&self, symbol: &str) -> &[Action] {
        self.by_symbol.get(symbol).map_or(&[], Vec::as_slice)
    }

    /// Every symbol the table holds rows for, sorted.
    #[must_use]
    pub fn symbols(&self) -> Vec<&str> {
        let mut symbols: Vec<&str> = self.by_symbol.keys().map(String::as_str).collect();
        symbols.sort_unstable();
        symbols
    }

    /// Every row of `symbol` whose ex-date lies in `(after, through]`, of
    /// either tag: what a page may LABEL. Pass the previous session as
    /// `after` and the session as `through`, so an ex-date on a holiday
    /// applies at the next session. One hash and two binary searches.
    #[must_use]
    pub fn labels(&self, symbol: &str, after: Option<Day>, through: Day) -> &[Action] {
        let rows = self.of(symbol);
        let start = after.map_or_else(
            || rows.partition_point(|row| row.ex_date < through),
            |after| rows.partition_point(|row| row.ex_date <= after),
        );
        let end = rows.partition_point(|row| row.ex_date <= through);
        rows.get(start..end).unwrap_or(&[])
    }

    /// The first row in `(after, through]` that may suppress a refusal:
    /// `nse` rows only. A measured row is never returned here
    /// (`a_measured_row_never_suppresses_a_refusal_or_decides_a_class`).
    #[must_use]
    pub fn excuse(&self, symbol: &str, after: Option<Day>, through: Day) -> Option<&Action> {
        self.labels(symbol, after, through)
            .iter()
            .find(|row| row.excuses())
    }

    /// The class an `nse` row in `(after, through]` decides; `None` when only
    /// measured rows (or none) lie there.
    #[must_use]
    pub fn class_of(&self, symbol: &str, after: Option<Day>, through: Day) -> Option<Kind> {
        self.labels(symbol, after, through)
            .iter()
            .find_map(Action::class)
    }
}

// ─────────────────────────────── the basis runs ───────────────────────────────

/// How a basis run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunEnd {
    /// The next basis day's interval lies more than [`STEP_PERMILLE`] from
    /// the last day's: D-3339's `basis_step`, a ratio step.
    Step,
    /// The next day lies within [`STEP_PERMILLE`] of the last day but
    /// outside the run's band: the ratio walked, it did not step. Not an
    /// action.
    Drift,
    /// The last run of the months read.
    Open,
}

impl RunEnd {
    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Step => "step",
            Self::Drift => "drift",
            Self::Open => "open",
        }
    }

    /// The end a word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        [Self::Step, Self::Drift, Self::Open]
            .into_iter()
            .find(|end| end.word() == word)
    }
}

/// One maximal run of one Zerodha/GDFL ratio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The share.
    pub symbol: String,
    /// The first day with a basis in the run.
    pub first: Day,
    /// The last day with a basis in the run.
    pub last: Day,
    /// The simplest fraction inside `[low, high]`.
    pub ratio: Ratio,
    /// The run's band, low end: the intersection of its days' allowance
    /// intervals, each widened by [`STEP_PERMILLE`].
    pub low: Ratio,
    /// The band's high end.
    pub high: Ratio,
    /// How many days with a basis the run holds.
    pub days: u32,
    /// How it ended.
    pub end: RunEnd,
}

impl Run {
    /// The row as the table's fields, in column order.
    #[must_use]
    pub fn fields(&self) -> Vec<String> {
        vec![
            self.symbol.clone(),
            self.first.to_string(),
            self.last.to_string(),
            self.ratio.to_string(),
            self.low.to_string(),
            self.high.to_string(),
            self.days.to_string(),
            self.end.word().to_owned(),
        ]
    }

    /// Whether `day` lies inside the run, both ends included.
    #[must_use]
    pub fn holds(&self, day: Day) -> bool {
        self.first <= day && day <= self.last
    }
}

/// Why a compare could not apply a run's factor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outside {
    /// The table holds no run for the share.
    NoRuns,
    /// The day lies before, between or after the share's runs: no measured
    /// factor holds there, and none is borrowed from a neighbour.
    Between {
        /// The last day of the run before, if any.
        before: Option<Day>,
        /// The first day of the run after, if any.
        after: Option<Day>,
    },
}

impl fmt::Display for Outside {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRuns => f.write_str("no_basis_run: the table holds no run for this share"),
            Self::Between { before, after } => write!(
                f,
                "outside_every_run: the run before ends {}, the run after starts {}",
                before.map_or_else(|| "-".to_owned(), |d| d.to_string()),
                after.map_or_else(|| "-".to_owned(), |d| d.to_string()),
            ),
        }
    }
}

/// The basis-run table, loaded: per symbol its runs sorted by first day,
/// behind one hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasisRuns {
    rows: usize,
    by_symbol: HashMap<String, Vec<Run>>,
}

impl BasisRuns {
    /// Builds the table from a parsed file: the loader's `build` for
    /// [`Table::BasisRuns`].
    ///
    /// # Errors
    ///
    /// A file whose standing is not `measured` (a basis run is never
    /// sourced), a malformed row, a ratio outside its own interval, two runs
    /// of one share that overlap, or more than [`MAX_RUNS_PER_SYMBOL`] runs
    /// for one share; each by name.
    pub fn build(parsed: &Parsed, path: &Path) -> Result<Self, Refusal> {
        let table = Table::BasisRuns;
        if parsed.receipt.standing != Standing::Measured {
            return Err(refusal(
                table,
                parsed,
                path,
                RefusalKind::Header,
                0,
                format!(
                    "status {}: a basis run is measured, not sourced",
                    parsed.receipt.standing.word()
                ),
            ));
        }
        let mut by_symbol: HashMap<String, Vec<Run>> = HashMap::new();
        for (line, row) in &parsed.rows {
            let bad = |kind, detail: String| refusal(table, parsed, path, kind, *line, detail);
            let [symbol, first, last, ratio, low, high, days, end] = row.as_slice() else {
                return Err(bad(RefusalKind::Row, "wrong field count".to_owned()));
            };
            let day = |text: &str| {
                parse_day(text)
                    .ok_or_else(|| bad(RefusalKind::Row, format!("{text:?} is not a day")))
            };
            let fraction = |text: &str| {
                Ratio::parse(text)
                    .ok_or_else(|| bad(RefusalKind::Row, format!("{text:?} is not a ratio")))
            };
            let run = Run {
                symbol: symbol.clone(),
                first: day(first)?,
                last: day(last)?,
                ratio: fraction(ratio)?,
                low: fraction(low)?,
                high: fraction(high)?,
                days: days
                    .parse()
                    .ok()
                    .filter(|days: &u32| *days > 0)
                    .ok_or_else(|| bad(RefusalKind::Row, format!("{days:?} is not a day count")))?,
                end: RunEnd::parse(end)
                    .ok_or_else(|| bad(RefusalKind::Row, format!("{end:?} is not an end")))?,
            };
            if run.first > run.last {
                return Err(bad(
                    RefusalKind::Row,
                    format!("run {}..{} runs backwards", run.first, run.last),
                ));
            }
            if !(run.low <= run.ratio && run.ratio <= run.high) {
                return Err(bad(
                    RefusalKind::Row,
                    format!(
                        "ratio {} lies outside its own interval [{}, {}]",
                        run.ratio, run.low, run.high
                    ),
                ));
            }
            let held = by_symbol.entry(symbol.clone()).or_default();
            if held.len() >= MAX_RUNS_PER_SYMBOL {
                return Err(bad(
                    RefusalKind::TooMany,
                    format!("{symbol:?} has more than {MAX_RUNS_PER_SYMBOL} runs"),
                ));
            }
            held.push(run);
        }
        for (symbol, runs) in &mut by_symbol {
            runs.sort_by(|a, b| a.first.cmp(&b.first));
            if let Some(pair) = runs.windows(2).find(|pair| match pair {
                [a, b] => a.last >= b.first,
                _ => false,
            }) && let [a, b] = pair
            {
                return Err(refusal(
                    table,
                    parsed,
                    path,
                    RefusalKind::Duplicate,
                    0,
                    format!(
                        "{symbol:?} has two runs that overlap: {}..{} and {}..{}",
                        a.first, a.last, b.first, b.last
                    ),
                ));
            }
        }
        Ok(Self {
            rows: parsed.rows.len(),
            by_symbol,
        })
    }

    /// How many runs the table holds.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Every run of `symbol`, by first day: one hash.
    #[must_use]
    pub fn of(&self, symbol: &str) -> &[Run] {
        self.by_symbol.get(symbol).map_or(&[], Vec::as_slice)
    }

    /// Every symbol the table holds runs for, sorted.
    #[must_use]
    pub fn symbols(&self) -> Vec<&str> {
        let mut symbols: Vec<&str> = self.by_symbol.keys().map(String::as_str).collect();
        symbols.sort_unstable();
        symbols
    }

    /// The run of `symbol` that holds `day`, or where the day lies between
    /// runs. One hash and one binary search (at most twelve comparisons).
    ///
    /// # Errors
    ///
    /// [`Outside`], by name: no run for the share, or the day outside every
    /// run (before the first, in a gap between two, or after the last).
    pub fn run_on(&self, symbol: &str, day: Day) -> Result<&Run, Outside> {
        let runs = self.of(symbol);
        if runs.is_empty() {
            return Err(Outside::NoRuns);
        }
        let at = runs.partition_point(|run| run.first <= day);
        let before = at.checked_sub(1).and_then(|i| runs.get(i));
        match before {
            Some(run) if run.holds(day) => Ok(run),
            _ => Err(Outside::Between {
                before: before.map(|run| run.last),
                after: runs.get(at).map(|run| run.first),
            }),
        }
    }

    /// A cross-vendor compare's factor: the run's ratio, ONLY inside its run
    /// (`a_compare_inside_a_run_applies_its_factor`). A Zerodha level on
    /// GDFL's raw basis is `level / ratio` ([`Ratio::to_gdfl`]).
    ///
    /// # Errors
    ///
    /// [`Outside`] wherever no run holds the day: no factor is borrowed from
    /// the run before or after.
    pub fn factor_on(&self, symbol: &str, day: Day) -> Result<Ratio, Outside> {
        self.run_on(symbol, day).map(|run| run.ratio)
    }
}

// ─────────────────────────────── the fold ───────────────────────────────

/// One stock-day with a basis: its day and its allowance interval
/// (DATA-1's `FactorPair::basis`, as a positive exact fraction pair).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasisDay {
    /// The day.
    pub day: Day,
    /// The interval's low end.
    pub low: Ratio,
    /// The interval's high end.
    pub high: Ratio,
}

/// Why a share's days could not be folded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldError {
    /// A day was not after the one before it (a duplicate or out of order).
    OutOfOrder {
        /// The day before.
        before: Day,
        /// The day that followed it.
        then: Day,
    },
    /// A day's interval runs backwards.
    Backwards(Day),
    /// The share holds more runs than [`MAX_RUNS_PER_SYMBOL`].
    TooMany,
    /// A day's interval has terms too large to widen exactly.
    TooLarge(Day),
}

impl fmt::Display for FoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfOrder { before, then } => {
                write!(f, "day {then} follows {before}: days must ascend, once each")
            }
            Self::Backwards(day) => write!(f, "day {day}: its interval runs backwards"),
            Self::TooMany => write!(f, "more than {MAX_RUNS_PER_SYMBOL} runs"),
            Self::TooLarge(day) => write!(
                f,
                "day {day}: its interval's terms are too large to widen exactly"
            ),
        }
    }
}

/// A run being folded.
#[derive(Debug, Clone, Copy)]
struct Open {
    first: Day,
    last: Day,
    low: Ratio,
    high: Ratio,
    days: u32,
    /// The last day's own interval, for the step-or-drift judgement.
    last_low: Ratio,
    last_high: Ratio,
}

/// `ratio x (1000 + STEP_PERMILLE) / 1000`: one end of the tolerance band.
fn widen_up(ratio: Ratio) -> Option<Ratio> {
    ratio.times(Ratio::new(1_000 + u128::from(STEP_PERMILLE), 1_000)?)
}

/// `ratio x 1000 / (1000 + STEP_PERMILLE)`: the other end.
fn widen_down(ratio: Ratio) -> Option<Ratio> {
    ratio.times(Ratio::new(1_000, 1_000 + u128::from(STEP_PERMILLE))?)
}

/// A day's interval widened by [`STEP_PERMILLE`] on each side: the band a
/// run's ratio may lie in and still be this day's.
fn widened(day: &BasisDay) -> Result<(Ratio, Ratio), FoldError> {
    widen_down(day.low)
        .zip(widen_up(day.high))
        .ok_or(FoldError::TooLarge(day.day))
}

impl Open {
    fn start(day: &BasisDay) -> Result<Self, FoldError> {
        let (low, high) = widened(day)?;
        Ok(Self {
            first: day.day,
            last: day.day,
            low,
            high,
            days: 1,
            last_low: day.low,
            last_high: day.high,
        })
    }

    fn close(self, symbol: &str, end: RunEnd) -> Run {
        Run {
            symbol: symbol.to_owned(),
            first: self.first,
            last: self.last,
            // The intersection is non-empty by construction, so a simplest
            // fraction exists; its low end stands in only if the walk ever
            // overflowed, which terms of 10^18 cannot.
            ratio: Ratio::simplest_between(self.low, self.high).unwrap_or(self.low),
            low: self.low,
            high: self.high,
            days: self.days,
            end,
        }
    }
}

/// The maximal runs of one Zerodha/GDFL ratio in `days` (ascending, once
/// each): a run ends `step` where the next day's interval lies more than
/// [`STEP_PERMILLE`] from the last day's (D-3339's `basis_step`); else it
/// grows while its band (the intersection of its days' intervals, each
/// widened by [`STEP_PERMILLE`]) is not empty, and ends `drift` where it
/// empties. O(days), one constant step a day. A share with no basis day has no run.
///
/// # Errors
///
/// [`FoldError`], by name.
pub fn fold_runs(
    symbol: &str,
    days: impl IntoIterator<Item = BasisDay>,
) -> Result<Vec<Run>, FoldError> {
    let mut folder = Folder::new(symbol);
    for day in days {
        folder.push(day)?;
    }
    Ok(folder.finish())
}

/// [`fold_runs`] one day at a time, for a caller that reads its days a
/// month at a time and keeps one folder per share between months.
#[derive(Debug, Clone)]
pub struct Folder {
    symbol: String,
    runs: Vec<Run>,
    open: Option<Open>,
}

impl Folder {
    /// A folder for `symbol` that has seen no day.
    #[must_use]
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_owned(),
            runs: Vec::new(),
            open: None,
        }
    }

    /// Folds one more day in: O(1).
    ///
    /// # Errors
    ///
    /// [`FoldError`], by name; the folder is unchanged by a refused day.
    pub fn push(&mut self, day: BasisDay) -> Result<(), FoldError> {
        if day.low > day.high {
            return Err(FoldError::Backwards(day.day));
        }
        let Some(mut run) = self.open else {
            self.open = Some(Open::start(&day)?);
            return Ok(());
        };
        if day.day <= run.last {
            return Err(FoldError::OutOfOrder {
                before: run.last,
                then: day.day,
            });
        }
        let (band_low, band_high) = widened(&day)?;
        // A STEP FIRST: the day's interval lies more than STEP_PERMILLE from
        // the last day's, D-3339's `basis_step`; then the band.
        let stepped = day.low > widen_up(run.last_high).ok_or(FoldError::TooLarge(day.day))?
            || widen_up(day.high).ok_or(FoldError::TooLarge(day.day))? < run.last_low;
        let low = run.low.max(band_low);
        let high = run.high.min(band_high);
        if !stepped && low <= high {
            run.last = day.day;
            run.low = low;
            run.high = high;
            run.days = run.days.saturating_add(1);
            run.last_low = day.low;
            run.last_high = day.high;
            self.open = Some(run);
            return Ok(());
        }
        if self.runs.len() >= MAX_RUNS_PER_SYMBOL.saturating_sub(1) {
            return Err(FoldError::TooMany);
        }
        let end = if stepped { RunEnd::Step } else { RunEnd::Drift };
        self.runs.push(run.close(&self.symbol, end));
        self.open = Some(Open::start(&day)?);
        Ok(())
    }

    /// The runs, the last one `open`; none when no day was folded.
    #[must_use]
    pub fn finish(mut self) -> Vec<Run> {
        if let Some(run) = self.open.take() {
            self.runs.push(run.close(&self.symbol, RunEnd::Open));
        }
        self.runs
    }
}

// ─────────────────────────────── steps, ledger, checker ───────────────────────────────

/// One ratio step between two consecutive runs of a share that ended
/// `step`: the ex-date lies in `(after, ex_date]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The share.
    pub symbol: String,
    /// The last day on the old basis.
    pub after: Day,
    /// The first day on the new basis.
    pub ex_date: Day,
    /// The old run's ratio.
    pub from: Ratio,
    /// The new run's ratio.
    pub to: Ratio,
    /// The factor's interval: `[new.low / old.high, new.high / old.low]`.
    pub low: Ratio,
    /// Its high end.
    pub high: Ratio,
    /// The simplest fraction inside `[low, high]`.
    pub factor: Ratio,
}

/// Every step of `runs` (one share's, by first day). O(runs).
#[must_use]
pub fn steps_of(runs: &[Run]) -> Vec<Step> {
    runs.windows(2)
        .filter_map(|pair| {
            let [old, new] = pair else { return None };
            if old.end != RunEnd::Step {
                return None;
            }
            let low = new.low.over(old.high)?;
            let high = new.high.over(old.low)?;
            Some(Step {
                symbol: old.symbol.clone(),
                after: old.last,
                ex_date: new.first,
                from: old.ratio,
                to: new.ratio,
                low,
                high,
                factor: Ratio::simplest_between(low, high)?,
            })
        })
        .collect()
}

/// The measured rows the steps of `runs` make: one per step, tagged
/// `measured`, kind `unclassed`, its window and its evidence named. A
/// measured row LABELS; it never excuses and never classes.
#[must_use]
pub fn measured_rows(runs: &BasisRuns, generation: u64) -> Vec<Action> {
    let mut rows = Vec::new();
    for symbol in runs.symbols() {
        for step in steps_of(runs.of(symbol)) {
            rows.push(Action {
                symbol: step.symbol.clone(),
                isin: None,
                ex_date: step.ex_date,
                kind: Kind::Unclassed,
                ratio: step.factor,
                tag: Tag::Measured,
                after: Some(step.after),
                evidence: format!(
                    "basis_runs generation {generation}: ratio {} to {}, factor in [{}, {}]",
                    step.from, step.to, step.low, step.high
                ),
            });
        }
    }
    rows
}

/// The measured corporate-action table drawn from `runs`, as a draft for
/// [`crate::reference_data::write`].
#[must_use]
pub fn measured_draft(runs: &BasisRuns, generation: u64, sha256: &str, fetched: Day) -> Draft {
    Draft {
        table: Table::CorporateActions,
        source: format!(
            "measured: the ratio steps of basis_runs generation {generation} (cli basis-runs over \
             DATA-1's per stock-day factor); not an NSE file"
        ),
        source_sha256: Some(sha256.to_owned()),
        fetched,
        standing: Standing::Measured,
        decision: DECISION.to_owned(),
        rows: measured_rows(runs, generation)
            .iter()
            .map(Action::fields)
            .collect(),
    }
}

/// What the checker found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A share's ratio stepped and no row lies in the step's window.
    NoRow(Step),
    /// Rows lie in the window and their factors' product is outside the
    /// step's interval.
    FactorDiffers {
        /// The step.
        step: Step,
        /// The product of the rows' ratios.
        product: Ratio,
    },
    /// A row that moves the price lies inside one run: the basis never moved
    /// across it.
    RowWithoutStep(Action),
}

impl Finding {
    /// The wire word.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::NoRow(_) => "no_row",
            Self::FactorDiffers { .. } => "factor_differs",
            Self::RowWithoutStep(_) => "row_without_step",
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRow(step) => write!(
                f,
                "no_row {} ({}, {}]: ratio {} to {}, factor {} in [{}, {}], and no corporate-action \
                 row lies there",
                step.symbol, step.after, step.ex_date, step.from, step.to, step.factor, step.low,
                step.high
            ),
            Self::FactorDiffers { step, product } => write!(
                f,
                "factor_differs {} ({}, {}]: the rows multiply to {product}, the step's factor \
                 lies in [{}, {}]",
                step.symbol, step.after, step.ex_date, step.low, step.high
            ),
            Self::RowWithoutStep(row) => write!(
                f,
                "row_without_step {} {} {} {} ({}): the basis did not move across it",
                row.symbol,
                row.ex_date,
                row.kind.word(),
                row.ratio,
                row.tag.word()
            ),
        }
    }
}

/// Every ratio step with no row (or with rows of another factor), and every
/// price-moving row inside one run, in symbol then day order. O(runs + rows).
#[must_use]
pub fn check(runs: &BasisRuns, actions: &CorporateActions) -> Vec<Finding> {
    let mut found = Vec::new();
    for symbol in runs.symbols() {
        for step in steps_of(runs.of(symbol)) {
            let rows = actions.labels(symbol, Some(step.after), step.ex_date);
            if rows.is_empty() {
                found.push(Finding::NoRow(step));
                continue;
            }
            let product = rows
                .iter()
                .try_fold(Ratio::ONE, |acc, row| acc.times(row.ratio));
            match product {
                Some(product) if step.low <= product && product <= step.high => {}
                Some(product) => found.push(Finding::FactorDiffers { step, product }),
                None => found.push(Finding::FactorDiffers {
                    step,
                    product: Ratio::ONE,
                }),
            }
        }
        for row in actions.of(symbol) {
            if row.ratio == Ratio::ONE {
                continue;
            }
            // One binary search a row: the run holding the ex-date, if any,
            // and whether the day before it lies in the same run.
            if runs
                .run_on(symbol, row.ex_date)
                .is_ok_and(|run| run.first < row.ex_date)
            {
                found.push(Finding::RowWithoutStep(row.clone()));
            }
        }
    }
    found
}

#[cfg(test)]
#[path = "corporate_actions_tests.rs"]
mod tests;
