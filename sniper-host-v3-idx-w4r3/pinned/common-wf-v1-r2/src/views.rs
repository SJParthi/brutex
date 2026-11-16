//! Views as data (4.2, P13-P18): the eligibility words E1-E7, the key criteria K1-K17 (K16 TRADE_GAP and K17 MONEY_GAP name
//! P17's two gap terms), keys in fixed arrays, the TICKETS of P21 (a ticket per path; dominance; the role read first, U1), and
//! `Lists`: per (partition, view) the k smallest keys (lifted from evalcore `views.rs:73-126`: an O(1) early reject, an O(k)
//! insert, a merge that is associative and commutative), the tie counter and the counts.

use std::collections::{BTreeMap, BTreeSet};

use crate::acc::{Acc, Facts, Learn, Steps};
use crate::rules::{DeclaredGrid, GridRole, Provenance, RuleProvenance, RunRules, parse_prov};
use crate::verdict::PickKind;
use crate::windows::{WinKind, WinRow, Windows};
use crate::{MAX_FLOORS, MAX_KEY, MAX_TICKETS, Mk, NO_GRID, Partition, RED, ReadSet, SetId, ViewId, WfErr, WinId, add, as_i64};

/// The host's canonical combo code (D13): the last tie-break, never the offer order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(pub [u64; 4]);

/// A view's role (P13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// THE pick of its split window.
    Headline,
    /// Beside the headline (the F dials, -N, -G, exact rows).
    Beside,
    /// The closest misses (P17).
    Miss,
    /// Window HS only.
    Hindsight,
    /// A way-A view (a labelled comparison).
    Comparison,
}
/// An eligibility word (P14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elig {
    /// At least one booked learning trade.
    E1,
    /// Every learning unit booked-positive.
    E2,
    /// At most one learning unit not booked-positive.
    E3,
    /// EVERY learning unit holds at least n booked trades (an untouched unit counts 0, R6).
    E4(u64),
    /// At least x learning trades a walked learning session.
    E5(u64),
    /// The read set (P21): every view checks it.
    E6,
    /// Not E2 (MISS views).
    E7,
}
/// K3's form (SW16).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// 0 when every counted learning month is positive (standard, D31).
    Z,
    /// Minus the worst month always.
    Raw,
}
/// K5's base (SW12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base {
    /// One step per trade (standard, D15).
    Trade,
    /// One step per session net.
    Session,
}
/// A key criterion (P15; smaller ranks first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crit {
    /// Learning units not booked-positive.
    K1,
    /// Learning months not positive.
    K2,
    /// The worst-month term.
    K3(Form),
    /// Minus the learning net.
    K4,
    /// The max drawdown, its base and optional bucket (bp of the base capital).
    K5(Base, Option<i64>),
    /// The largest worst run-down, bp.
    K6,
    /// max(0, -the worst trade).
    K7,
    /// The sum of -net over negative learning months.
    K8,
    /// 0 when every counted learning month is positive, else 1.
    K9,
    /// Learning trades.
    K10,
    /// Minus the win rate, bp.
    K11,
    /// Minus the profit factor, thousandths (no loser = the largest value).
    K12,
    /// Learning months below the bar (paisa).
    K13(i64),
    /// 0 every learning unit positive / 1 not / 2 no trade.
    K14,
    /// A host part (its index in the declared host names).
    K15(u8),
    /// P17's TRADE GAP: learning units with no trade.
    K16,
    /// P17's MONEY GAP: paisa to bring every failing learning unit to +1 paisa (an empty unit adds 1 paisa).
    K17,
}
impl Crit {
    /// The printed name (W17).
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::K1 => "K1 YEARS_NOT_POS",
            Self::K2 => "K2 MONTHS_NOT_POS",
            Self::K3(_) => "K3 WORST_MONTH_W",
            Self::K4 => "K4 NEG_NET",
            Self::K5(..) => "K5 MAX_DD",
            Self::K6 => "K6 WORST_RD",
            Self::K7 => "K7 WORST_TRADE_LOSS",
            Self::K8 => "K8 LOSING_MONTH_SUM",
            Self::K9 => "K9 EVERY_MONTH_TIER",
            Self::K10 => "K10 TRADES",
            Self::K11 => "K11 NEG_WIN_RATE",
            Self::K12 => "K12 NEG_PROFIT_FACTOR",
            Self::K13(_) => "K13 MONTHS_BELOW_BAR",
            Self::K14 => "K14 YEARS_CLASS",
            Self::K15(_) => "K15 HOST",
            Self::K16 => "K16 TRADE_GAP",
            Self::K17 => "K17 MONEY_GAP",
        }
    }
    /// Is the value money (printed with `fmt_rs`)?
    #[must_use]
    pub const fn money(&self) -> bool {
        matches!(self, Self::K3(_) | Self::K4 | Self::K7 | Self::K8 | Self::K17 | Self::K5(_, None))
    }
    const fn needs_trades(&self) -> bool {
        matches!(self, Self::K5(..) | Self::K6 | Self::K7)
    }
}

/// One view (P13).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// Its id.
    pub id: String,
    /// Its window.
    pub win: WinId,
    /// Its role.
    pub role: Role,
    /// How many entries it keeps.
    pub k: usize,
    /// Eligibility, in order.
    pub elig: Vec<Elig>,
    /// The key, in order (CODE implied last).
    pub key: Vec<Crit>,
    /// Its label.
    pub label: String,
    /// Its provenance: its own row's, else the file version's header (A2).
    pub prov: RuleProvenance,
    /// Its own row's provenance, if written.
    pub own_prov: Option<RuleProvenance>,
}

/// A views file version (4.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Views {
    /// The signed header row's provenance (Z1).
    pub header: RuleProvenance,
    /// The views, in file order.
    pub views: Vec<View>,
    /// The distinct K13 bars (at most 4).
    pub bars: Vec<i64>,
    /// The distinct E4 floors (at most 8).
    pub floors: Vec<u64>,
    /// The declared host part names (K15).
    pub host_names: Vec<String>,
    /// The identity hash of the file.
    pub ident: u64,
}

fn parse_e(w: &str) -> Option<Elig> {
    let arg = |p: &str| w.strip_prefix(p).and_then(|x| x.strip_suffix(')')).filter(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit())).and_then(|x| x.parse().ok());
    match w {
        "E1" => Some(Elig::E1),
        "E2" => Some(Elig::E2),
        "E3" => Some(Elig::E3),
        "E6" => Some(Elig::E6),
        "E7" => Some(Elig::E7),
        _ => arg("E4(").map(Elig::E4).or_else(|| arg("E5(").map(Elig::E5)),
    }
}
enum KParse {
    Ok(Crit, bool),
    Undeclared,
    Bad,
}
fn parse_k(w: &str, base: i64, host: &[&str]) -> KParse {
    let (w, alt) = w.strip_suffix("@alt").map_or((w, false), |x| (x, true));
    let num = |s: &str| -> Option<i64> { if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) { s.parse().ok() } else { None } };
    let c = match w {
        "K1" => Some(Crit::K1),
        "K2" => Some(Crit::K2),
        "K4" => Some(Crit::K4),
        "K6" => Some(Crit::K6),
        "K7" => Some(Crit::K7),
        "K8" => Some(Crit::K8),
        "K9" => Some(Crit::K9),
        "K10" => Some(Crit::K10),
        "K11" => Some(Crit::K11),
        "K12" => Some(Crit::K12),
        "K14" => Some(Crit::K14),
        "K16" => Some(Crit::K16),
        "K17" => Some(Crit::K17),
        "K3(Z)" => Some(Crit::K3(Form::Z)),
        "K3(RAW)" => Some(Crit::K3(Form::Raw)),
        "K5(TRADE)" => Some(Crit::K5(Base::Trade, None)),
        "K5(SESSION)" => Some(Crit::K5(Base::Session, None)),
        _ => None,
    };
    if let Some(c) = c {
        return KParse::Ok(c, alt);
    }
    if let Some(x) = w.strip_prefix("K5(").and_then(|x| x.strip_suffix("))")) {
        let (b, rest) = x.split_once(",BUCKET(").unwrap_or(("", ""));
        let base_k = match b {
            "TRADE" => Some(Base::Trade),
            "SESSION" => Some(Base::Session),
            _ => None,
        };
        if let (Some(bk), Some(n)) = (base_k, num(rest)) {
            return KParse::Ok(Crit::K5(bk, Some(n)), alt);
        }
        return KParse::Bad;
    }
    if let Some(x) = w.strip_prefix("K13(bp(").and_then(|x| x.strip_suffix("))")) {
        return num(x).and_then(|n| base.checked_mul(n)).map_or(KParse::Bad, |b| KParse::Ok(Crit::K13(b / 10_000), alt));
    }
    if let Some(x) = w.strip_prefix("K13(").and_then(|x| x.strip_suffix(')')) {
        return num(x).map_or(KParse::Bad, |b| KParse::Ok(Crit::K13(b), alt));
    }
    if let Some(x) = w.strip_prefix("HOST(").and_then(|x| x.strip_suffix(')')) {
        if x.is_empty() || !x.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
            return KParse::Bad;
        }
        return host.iter().position(|h| *h == x).and_then(|i| u8::try_from(i).ok()).map_or(KParse::Undeclared, |i| KParse::Ok(Crit::K15(i), alt));
    }
    KParse::Bad
}

impl Views {
    /// Parses and validates a views file version (4.2): the signed header row first (Z1), then the column row, then the views.
    ///
    /// # Errors
    /// `BadRow(V, row, rule)`: 1 a column count, role or unknown word; 2 not exactly one HEADLINE per split window; 3 k out of
    /// 1..=1000; 4 more than 12 criteria; 5 more than 4 bars; 6 an `@alt` tag (P15); 7 no header row; 8 an unknown window;
    /// 9 an undeclared HOST part; 10 a duplicate id; 11 no E6 (no opt-out); 12 a bad provenance; 13 the column row; 14 a role
    /// on the wrong window kind; 15 more than 8 E4 floors.
    pub fn from_text(text: &str, rows: &[WinRow], r: &RunRules, host: &[&str]) -> Result<Self, WfErr> {
        let bad = |row: usize, rule: u8| WfErr::BadRow { file: b'V', row: u32::try_from(row).unwrap_or(u32::MAX), rule };
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        let head = lines.first().filter(|l| l.starts_with("#views_provenance\t")).ok_or_else(|| bad(0, 7))?;
        if lines.len() < 2 {
            return Err(bad(0, 7));
        }
        let header = head.split('\t').nth(1).and_then(parse_prov).filter(|p| !matches!(p, RuleProvenance::RuledKnowingly { .. })).ok_or_else(|| bad(0, 12))?;
        if lines.get(1).copied() != Some("id\twindow\trole\tk\teligibility\tkey\tlabel\tprovenance") {
            return Err(bad(0, 13));
        }
        let mut views: Vec<View> = Vec::new();
        let mut bars: Vec<i64> = Vec::new();
        let mut floors: Vec<u64> = Vec::new();
        for (i, l) in lines.iter().skip(2).enumerate() {
            let n = i + 1;
            let c: Vec<&str> = l.split('\t').collect();
            if c.len() != 8 {
                return Err(bad(n, 1));
            }
            let col = |k: usize| c.get(k).copied().unwrap_or("");
            let w = rows.iter().find(|x| x.id == col(1)).ok_or_else(|| bad(n, 8))?;
            let role = match col(2) {
                "HEADLINE" => Role::Headline,
                "BESIDE" => Role::Beside,
                "MISS" => Role::Miss,
                "HINDSIGHT" => Role::Hindsight,
                "COMPARISON" => Role::Comparison,
                _ => return Err(bad(n, 1)),
            };
            let ok_role = matches!((w.kind, role), (WinKind::Split, Role::Headline | Role::Beside | Role::Miss) | (WinKind::Hindsight, Role::Hindsight) | (WinKind::WayA, Role::Comparison));
            if !ok_role {
                return Err(bad(n, 14));
            }
            let k: usize = if !col(3).is_empty() && col(3).bytes().all(|b| b.is_ascii_digit()) { col(3).parse().map_err(|_| bad(n, 3))? } else { return Err(bad(n, 3)) };
            if !(1..=1000).contains(&k) {
                return Err(bad(n, 3));
            }
            let elig: Vec<Elig> = col(4).split(' ').map(parse_e).collect::<Option<Vec<_>>>().ok_or_else(|| bad(n, 1))?;
            if !elig.contains(&Elig::E6) {
                return Err(bad(n, 11));
            }
            let parsed: Vec<KParse> = col(5).split(' ').map(|x| parse_k(x, r.base_capital, host)).collect();
            if parsed.iter().any(|x| matches!(x, KParse::Bad)) {
                return Err(bad(n, 1));
            }
            if parsed.iter().any(|x| matches!(x, KParse::Undeclared)) {
                return Err(bad(n, 9));
            }
            if parsed.len() > MAX_KEY {
                return Err(bad(n, 4));
            }
            if parsed.iter().any(|x| matches!(x, KParse::Ok(_, true))) {
                return Err(bad(n, 6));
            }
            let key: Vec<Crit> = parsed.into_iter().filter_map(|x| if let KParse::Ok(c, _) = x { Some(c) } else { None }).collect();
            for c in &key {
                if let Crit::K13(b) = c
                    && !bars.contains(b)
                {
                    bars.push(*b);
                }
            }
            for e in &elig {
                if let Elig::E4(f) = e
                    && !floors.contains(f)
                {
                    floors.push(*f);
                }
            }
            let own_prov = if col(7) == "-" { None } else { Some(parse_prov(col(7)).filter(|p| !matches!(p, RuleProvenance::RuledKnowingly { .. })).ok_or_else(|| bad(n, 12))?) };
            if views.iter().any(|v| v.id == col(0)) {
                return Err(bad(n, 10));
            }
            views.push(View { id: col(0).to_owned(), win: WinId(w.idx), role, k, elig, key, label: col(6).to_owned(), prov: own_prov.unwrap_or(header), own_prov });
        }
        for w in rows.iter().filter(|w| w.kind == WinKind::Split) {
            if views.iter().filter(|v| v.win == WinId(w.idx) && v.role == Role::Headline).count() != 1 {
                return Err(bad(0, 2));
            }
        }
        if bars.len() > crate::MAX_BARS {
            return Err(bad(0, 5));
        }
        if floors.len() > MAX_FLOORS {
            return Err(bad(0, 15));
        }
        Ok(Self { header, views, bars, floors, host_names: host.iter().map(|h| (*h).to_owned()).collect(), ident: crate::text_hash(text) })
    }
}

/// One path by which a subject reached the offer (P21): its grid (index into `RunRules::grids`, or `NO_GRID`), this run's last
/// shaping stage and the read set of every stage on the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    /// The grid.
    pub grid: u8,
    /// The stage id.
    pub stage: u8,
    /// The stages' read set.
    pub set: ReadSet,
}
/// The tickets of one offer (at most 4 non-dominated paths) with the push counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tickets {
    n: u8,
    t: [Ticket; MAX_TICKETS],
    /// Tickets dropped as dominated (Y6).
    pub dominated: u32,
    /// NO_GRID tickets pushed (S1).
    pub no_grid: u32,
    /// FIXED tickets refused at push (U1).
    pub fixed_offered: u32,
}
impl Default for Tickets {
    fn default() -> Self {
        Self::new()
    }
}
fn effective(t: &Ticket, grids: &[DeclaredGrid]) -> ReadSet {
    if cfg!(feature = "m27") {
        return t.set;
    }
    grids.get(usize::from(t.grid)).map_or(t.set, |g| t.set.hull(g.read_set()))
}
impl Tickets {
    /// No ticket.
    #[must_use]
    pub const fn new() -> Self {
        Self { n: 0, t: [Ticket { grid: NO_GRID, stage: 0, set: ReadSet::EMPTY }; MAX_TICKETS], dominated: 0, no_grid: 0, fixed_offered: 0 }
    }
    /// The kept tickets.
    #[must_use]
    pub fn kept(&self) -> &[Ticket] {
        self.t.get(..usize::from(self.n)).unwrap_or(&[])
    }
    fn remove_where(&mut self, f: impl Fn(&Ticket) -> bool) -> u32 {
        let mut out = [Ticket { grid: NO_GRID, stage: 0, set: ReadSet::EMPTY }; MAX_TICKETS];
        let mut k = 0usize;
        let mut dropped = 0u32;
        for t in self.kept() {
            if f(t) {
                dropped += 1;
            } else if let Some(x) = out.get_mut(k) {
                *x = *t;
                k += 1;
            }
        }
        self.t = out;
        self.n = u8::try_from(k).unwrap_or(0);
        dropped
    }
    /// Pushes one path (Y6; U1 / Q1: the grid ROLE first). A FIXED grid's ticket is refused before any dominance; a NO_GRID
    /// ticket counts as the full period (it never drops a SEARCH ticket); dominance among SEARCH tickets only (a ticket whose
    /// effective set contains another's is dropped); a fifth non-dominated path is refused.
    ///
    /// # Errors
    /// `FixedOffered`, `TooManyTickets`, `BadRow(R, 0, 9)` an undeclared grid.
    pub fn push(&mut self, t: Ticket, r: &RunRules) -> Result<(), WfErr> {
        if t.grid == NO_GRID && !cfg!(feature = "m42") {
            self.no_grid += 1;
            if self.n > 0 {
                self.dominated += 1;
                return Ok(());
            }
            return self.store(t);
        }
        let fixed_role = if t.grid == NO_GRID { false } else { r.grids.get(usize::from(t.grid)).ok_or(WfErr::BadRow { file: b'R', row: 0, rule: 9 })?.role == GridRole::Fixed };
        if fixed_role && !cfg!(feature = "m29") {
            self.fixed_offered += 1;
            return Err(WfErr::FixedOffered);
        }
        let e = effective(&t, &r.grids);
        self.dominated += self.remove_where(|k| k.grid == NO_GRID);
        if self.kept().iter().any(|k| e.contains(&effective(k, &r.grids))) {
            self.dominated += 1;
            return Ok(());
        }
        self.dominated += self.remove_where(|k| effective(k, &r.grids).contains(&e));
        self.store(t)
    }
    fn store(&mut self, t: Ticket) -> Result<(), WfErr> {
        let slot = self.t.get_mut(usize::from(self.n)).ok_or(WfErr::TooManyTickets)?;
        *slot = t;
        self.n += 1;
        Ok(())
    }
}

/// A key: the criteria values in a fixed array (no Vec, 6.5) and the code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    /// The criteria values.
    pub parts: [i64; MAX_KEY],
    /// How many are used.
    pub n: u8,
    /// The code.
    pub code: Code,
}
impl Key {
    fn used(&self) -> &[i64] {
        self.parts.get(..usize::from(self.n)).unwrap_or(&[])
    }
    /// The total order: the criteria, then the code (the red reading: the code is ignored, ties by offer order).
    #[must_use]
    pub fn cmp_key(&self, o: &Self) -> std::cmp::Ordering {
        self.used().cmp(o.used()).then_with(|| if RED || cfg!(feature = "m5") { std::cmp::Ordering::Equal } else { self.code.cmp(&o.code) })
    }
}

/// The learning summary of a kept entry (W3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LearnSummary {
    /// Learning trades.
    pub trades: u64,
    /// Learning sessions with a trade.
    pub sessions: u64,
    /// The learning net.
    pub net: i64,
    /// The learning drawdown, TRADE base.
    pub dd_trade: i64,
    /// The learning drawdown, SESSION base.
    pub dd_session: i64,
    /// The learning mark-to-market drawdown.
    pub dd_mtm: i64,
    /// The learning worst run-down.
    pub worst_rd: i64,
    /// Learning months not positive.
    pub months_not_pos: u64,
    /// The learning fingerprint (P18).
    pub fp: u64,
    /// The last mark at or before the learning end (R3).
    pub mark_end: Option<i64>,
}

/// One kept entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    /// Its key.
    pub key: Key,
    /// What kind of pick it is (set at offer, Q2).
    pub kind: PickKind,
    /// Its facts (year units, masks, identity).
    pub facts: Facts,
    /// Its learning summary.
    pub learn: LearnSummary,
    /// Its tickets.
    pub tickets: Tickets,
}

/// The counts of one (partition, view) (W7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Offers.
    pub offered: u64,
    /// Eligible offers.
    pub eligible: u64,
    /// Refused by the read set.
    pub rejected_read_set: u64,
    /// Refused: no grid (S1).
    pub rejected_no_grid: u64,
    /// Refused: a CHOSEN grid's months.
    pub rejected_chosen_grid: u64,
    /// Offers through a FIXED ticket (asserted 0 by hosts).
    pub fixed_offered: u64,
    /// Refused by E1.
    pub rejected_no_learn_trade: u64,
    /// Refused by E2 / E3 / E7.
    pub rejected_learn_year: u64,
    /// Refused by E4 / E5.
    pub rejected_floor: u64,
}
impl Counts {
    fn plus(&mut self, o: &Self) {
        self.offered += o.offered;
        self.eligible += o.eligible;
        self.rejected_read_set += o.rejected_read_set;
        self.rejected_no_grid += o.rejected_no_grid;
        self.rejected_chosen_grid += o.rejected_chosen_grid;
        self.fixed_offered += o.fixed_offered;
        self.rejected_no_learn_trade += o.rejected_no_learn_trade;
        self.rejected_learn_year += o.rejected_learn_year;
        self.rejected_floor += o.rejected_floor;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ViewList {
    items: Vec<Kept>,
    best: Option<([i64; MAX_KEY], u8)>,
    ties: u64,
    counts: Counts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ViewInSet {
    view: u16,
    win: WinId,
    kind: WinKind,
    learn: (Mk, Mk),
    win_chosen: Option<u32>,
}

/// The lists of ONE window set (R5): per (partition, view of this set) the kept entries, the ties and the counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lists {
    set: SetId,
    ident: u64,
    grids: Vec<DeclaredGrid>,
    parts: Vec<(u32, Option<u32>)>,
    base: i64,
    marks: bool,
    in_set: Vec<ViewInSet>,
    map: BTreeMap<(u32, u16), ViewList>,
    offered: BTreeSet<(u32, Code)>,
}

const EMPTY_KEPT: &[Kept] = &[];

impl Lists {
    /// The lists of window set `w` (R5).
    #[must_use]
    pub fn new(v: &Views, w: &Windows, r: &RunRules, ident: u64) -> Self {
        let in_set = v
            .views
            .iter()
            .enumerate()
            .filter_map(|(i, x)| {
                let s = w.spec(x.win)?;
                let row = w.row(x.win)?;
                let win_chosen = if let Some(RuleProvenance::Chosen { run, .. }) = row.prov { Some(run) } else { None };
                Some(ViewInSet { view: u16::try_from(i).ok()?, win: x.win, kind: s.kind, learn: s.learn.unwrap_or((0, -1)), win_chosen })
            })
            .collect();
        Self {
            set: w.set_id(),
            ident: crate::mix(ident ^ w.tab.ident ^ v.ident ^ r.ident),
            grids: r.grids.clone(),
            parts: r.partitions.iter().map(|p| (p.id, p.chosen_run())).collect(),
            base: r.base_capital,
            marks: r.mtm_marks == crate::rules::MtmMarks::SessionEnd,
            in_set,
            map: BTreeMap::new(),
            offered: BTreeSet::new(),
        }
    }

    /// Offers one subject ONCE per (this set's Lists, partition, code) to every view of the partition in this set (F3, 6.3).
    ///
    /// # Errors
    /// See [`Lists::offer_host`].
    pub fn offer(&mut self, v: &Views, p: Partition, t: &Tickets, code: Code, acc: &Acc) -> Result<Steps, WfErr> {
        self.offer_host(v, p, t, code, acc, &[])
    }

    /// `offer` with the host's K15 values (in `Views::host_names` order).
    ///
    /// # Errors
    /// `SetMismatch` (another set's `Acc`), `DoubleOffer` (feature `offer_once_check`), `NeedsTrades` (a month-fed `Acc` to a
    /// view reading K5-K7), `NoPartitionRow`, `FixedOffered` (tickets that hold a refused FIXED path; counted per view), `Overflow`.
    pub fn offer_host(&mut self, v: &Views, p: Partition, t: &Tickets, code: Code, acc: &Acc, host: &[i64]) -> Result<Steps, WfErr> {
        let mut st = Steps::default();
        if acc.set_id() != self.set {
            return Err(WfErr::SetMismatch);
        }
        if cfg!(feature = "offer_once_check") && !self.offered.insert((p.0, code)) {
            return Err(WfErr::DoubleOffer);
        }
        if acc.month_fed && self.in_set.iter().any(|x| v.views.get(usize::from(x.view)).is_some_and(|w| w.key.iter().any(Crit::needs_trades))) {
            return Err(WfErr::NeedsTrades);
        }
        let chosen = self.parts.iter().find(|x| x.0 == p.0).ok_or(WfErr::NoPartitionRow)?.1;
        let (grids, base, marks) = (self.grids.clone(), self.base, self.marks);
        let mut learns: Vec<(WinId, Learn)> = Vec::new();
        let fixed = t.fixed_offered > 0;
        let in_set = self.in_set.clone();
        for x in &in_set {
            let Some(view) = v.views.get(usize::from(x.view)) else { continue };
            let list = self.map.entry((p.0, x.view)).or_default();
            list.counts.offered += 1;
            st.offer_ops += 1;
            if fixed {
                list.counts.fixed_offered += 1;
                continue;
            }
            if let Some(reason) = refuse(x, t, &grids) {
                match reason {
                    Refusal::ReadSet => list.counts.rejected_read_set += 1,
                    Refusal::NoGrid => list.counts.rejected_no_grid += 1,
                    Refusal::ChosenGrid => list.counts.rejected_chosen_grid += 1,
                }
                continue;
            }
            let l = if let Some((_, l)) = learns.iter().find(|(w, _)| *w == x.win) {
                *l
            } else {
                let l = acc.learn(x.win)?;
                learns.push((x.win, l));
                l
            };
            match eligible(view, &l, acc) {
                Some(Why::NoLearn) => {
                    list.counts.rejected_no_learn_trade += 1;
                    continue;
                }
                Some(Why::Year) => {
                    list.counts.rejected_learn_year += 1;
                    continue;
                }
                Some(Why::Floor) => {
                    list.counts.rejected_floor += 1;
                    continue;
                }
                None => {}
            }
            list.counts.eligible += 1;
            // the O(1) early reject (6.3): a full list and a worse first criterion
            let first = view.key.first().map(|c| crit_val(c, &l, acc, base, marks, host)).transpose()?;
            if list.items.len() >= view.k
                && let (Some(f), Some(last)) = (first, list.items.last())
                && last.key.parts.first().is_some_and(|x| f > *x)
            {
                st.offer_ops += 1;
                continue;
            }
            let mut parts = [0i64; MAX_KEY];
            for (i, c) in view.key.iter().enumerate().take(MAX_KEY) {
                if let Some(s) = parts.get_mut(i) {
                    *s = crit_val(c, &l, acc, base, marks, host)?;
                }
            }
            let key = Key { parts, n: u8::try_from(view.key.len().min(MAX_KEY)).unwrap_or(0), code };
            st.offer_ops += u32::from(key.n);
            match list.best {
                Some((b, n)) if key.used() == b.get(..usize::from(n)).unwrap_or(&[]) => list.ties += 1,
                Some((b, n)) if key.used() < b.get(..usize::from(n)).unwrap_or(&[]) => {
                    list.best = Some((key.parts, key.n));
                    list.ties = 1;
                }
                None => {
                    list.best = Some((key.parts, key.n));
                    list.ties = 1;
                }
                Some(_) => {}
            }
            let at = list.items.iter().position(|it| key.cmp_key(&it.key) == std::cmp::Ordering::Less).unwrap_or(list.items.len());
            if at >= view.k {
                continue;
            }
            let kind = pick_kind(x, view, chosen);
            let lc = &l.cell;
            let learn = LearnSummary {
                trades: lc.trades,
                sessions: lc.sessions,
                net: lc.net,
                dd_trade: lc.dd_trade.max_dd,
                dd_session: lc.dd_session.max_dd,
                dd_mtm: lc.dd_mtm.max_dd,
                worst_rd: lc.worst_rd,
                months_not_pos: l.counted.saturating_sub(lc.m_pos),
                fp: lc.fp,
                mark_end: if lc.marked { Some(lc.mark_end) } else { None },
            };
            list.items.insert(at, Kept { key, kind, facts: acc.facts(p), learn, tickets: *t });
            list.items.truncate(view.k);
            st.offer_ops += 1;
        }
        if fixed {
            return Err(WfErr::FixedOffered);
        }
        Ok(st)
    }

    /// The kept entries of (p, view), smallest key first.
    #[must_use]
    pub fn kept(&self, p: Partition, view: ViewId) -> &[Kept] {
        self.map.get(&(p.0, view.0)).map_or(EMPTY_KEPT, |l| l.items.as_slice())
    }
    /// THE pick of (p, view).
    #[must_use]
    pub fn pick(&self, p: Partition, view: ViewId) -> Option<&Kept> {
        self.kept(p, view).first()
    }
    /// The counts of (p, view).
    #[must_use]
    pub fn counts(&self, p: Partition, view: ViewId) -> Counts {
        self.map.get(&(p.0, view.0)).map(|l| l.counts).unwrap_or_default()
    }
    /// Ties at #1 (P18): every eligible offer whose every key part equals #1's.
    #[must_use]
    pub fn ties(&self, p: Partition, view: ViewId) -> u64 {
        self.map.get(&(p.0, view.0)).map_or(0, |l| l.ties)
    }
    /// The list bound in bytes (F16): k x views x partitions x threads x the size of a kept entry.
    #[must_use]
    pub fn bound_bytes(v: &Views, partitions: u32, threads: u32) -> u64 {
        let per = u64::try_from(std::mem::size_of::<Kept>() + crate::MAX_PARTS * std::mem::size_of::<crate::acc::UnitCell>()).unwrap_or(u64::MAX);
        let k: u64 = v.views.iter().map(|x| u64::try_from(x.k).unwrap_or(0)).sum();
        k.saturating_mul(u64::from(partitions)).saturating_mul(u64::from(threads)).saturating_mul(per)
    }
    /// Merges another shard's lists of the same set and identity (associative, commutative; P13, MERGE).
    ///
    /// # Errors
    /// `SetMismatch`, `IdentityMismatch`.
    pub fn merge(&mut self, o: &Self, v: &Views) -> Result<(), WfErr> {
        if o.set != self.set {
            return Err(WfErr::SetMismatch);
        }
        if o.ident != self.ident {
            return Err(WfErr::IdentityMismatch);
        }
        for (k, ol) in &o.map {
            let kcap = v.views.get(usize::from(k.1)).map_or(0, |x| x.k);
            let l = self.map.entry(*k).or_default();
            l.counts.plus(&ol.counts);
            match (l.best, ol.best) {
                (None, Some(b)) => {
                    l.best = Some(b);
                    l.ties = ol.ties;
                }
                (Some((a, an)), Some((b, bn))) => {
                    let (aa, bb) = (a.get(..usize::from(an)).unwrap_or(&[]), b.get(..usize::from(bn)).unwrap_or(&[]));
                    if bb < aa {
                        l.best = Some((b, bn));
                        l.ties = ol.ties;
                    } else if bb == aa {
                        l.ties += ol.ties;
                    }
                }
                _ => {}
            }
            for it in &ol.items {
                let at = l.items.iter().position(|x| it.key.cmp_key(&x.key) == std::cmp::Ordering::Less).unwrap_or(l.items.len());
                if at < kcap {
                    l.items.insert(at, it.clone());
                    l.items.truncate(kcap);
                }
            }
        }
        for c in &o.offered {
            self.offered.insert(*c);
        }
        Ok(())
    }
}


enum Refusal {
    ReadSet,
    NoGrid,
    ChosenGrid,
}
/// P21's acceptance rule: some ticket's EFFECTIVE read set lies inside the view's learning months (HS views accept every ticket).
fn refuse(x: &ViewInSet, t: &Tickets, grids: &[DeclaredGrid]) -> Option<Refusal> {
    if x.kind == WinKind::Hindsight || RED {
        return None;
    }
    let (lf, lt) = x.learn;
    if t.kept().is_empty() {
        return Some(Refusal::NoGrid);
    }
    let mut reason = None;
    for k in t.kept() {
        if k.grid == NO_GRID {
            if (cfg!(feature = "m26") && k.set.is_empty()) || (cfg!(feature = "m37") && !k.set.is_empty() && lf <= k.set.from && k.set.to <= lt) {
                return None;
            }
            reason = reason.or(Some(Refusal::NoGrid));
            continue;
        }
        let Some(g) = grids.get(usize::from(k.grid)) else {
            reason = reason.or(Some(Refusal::NoGrid));
            continue;
        };
        let e = effective(k, grids);
        let inside = if cfg!(feature = "m16") { e.from <= lt && lf <= e.to } else { lf <= e.from && e.to <= lt };
        if e.is_empty() || inside {
            return None;
        }
        let g_out = matches!(g.provenance, Provenance::Chosen { set, .. } if !(lf <= set.from && set.to <= lt));
        reason = reason.or(Some(if g_out { Refusal::ChosenGrid } else { Refusal::ReadSet }));
    }
    reason
}

enum Why {
    NoLearn,
    Year,
    Floor,
}
fn eligible(v: &View, l: &Learn, acc: &Acc) -> Option<Why> {
    let all_pos = (l.booked & l.units) == l.units;
    for e in &v.elig {
        match e {
            Elig::E6 => {}
            Elig::E1 => {
                if l.cell.trades < 1 {
                    return Some(Why::NoLearn);
                }
            }
            Elig::E2 => {
                if !all_pos {
                    return Some(Why::Year);
                }
            }
            Elig::E3 => {
                if (l.units & !l.booked).count_ones() > 1 {
                    return Some(Why::Year);
                }
            }
            Elig::E4(n) => {
                let atl = acc.floor_index(*n).and_then(|i| l.atleast.get(i).copied()).unwrap_or(0);
                let fail = if cfg!(feature = "m30") { (l.units & l.touched & !atl) != 0 } else { (l.units & !l.touched) != 0 || (l.units & !atl) != 0 };
                if fail {
                    return Some(Why::Floor);
                }
            }
            Elig::E5(x) => {
                if u128::from(l.cell.trades) < u128::from(*x) * u128::from(l.walked_sessions) {
                    return Some(Why::Floor);
                }
            }
            Elig::E7 => {
                if all_pos {
                    return Some(Why::Year);
                }
            }
        }
    }
    None
}

fn worst_month(l: &Learn) -> i64 {
    if l.counted == 0 {
        0
    } else if l.cell.m_traded < l.counted {
        l.cell.m_worst.min(0)
    } else {
        l.cell.m_worst
    }
}
/// One criterion's value on the learning part (P15). O(1).
///
/// # Errors
/// `Overflow`.
pub fn crit_val(c: &Crit, l: &Learn, acc: &Acc, base_capital: i64, marks: bool, host: &[i64]) -> Result<i64, WfErr> {
    let cell = &l.cell;
    Ok(match c {
        Crit::K1 => i64::from((l.units & !l.booked).count_ones()),
        Crit::K2 => as_i64(l.counted.saturating_sub(cell.m_pos)),
        Crit::K3(f) => {
            if *f == Form::Z && cell.m_pos == l.counted {
                0
            } else {
                worst_month(l).checked_neg().ok_or(WfErr::Overflow)?
            }
        }
        Crit::K4 => cell.net.checked_neg().ok_or(WfErr::Overflow)?,
        Crit::K5(base, bucket) => {
            let mut d = if *base == Base::Session || RED { cell.dd_session.max_dd } else { cell.dd_trade.max_dd };
            if marks && !RED {
                d = d.max(cell.dd_mtm.max_dd);
            }
            if let Some(b) = bucket {
                let num = i128::from(d) * 10_000;
                let den = i128::from(base_capital) * i128::from(*b);
                if den <= 0 {
                    return Err(WfErr::Overflow);
                }
                d = i64::try_from((2 * num + den) / (2 * den)).map_err(|_| WfErr::Overflow)?;
            }
            d
        }
        Crit::K6 => cell.worst_rd,
        Crit::K7 => {
            if cell.worst_trade == i64::MAX || cell.worst_trade >= 0 {
                0
            } else {
                cell.worst_trade.checked_neg().ok_or(WfErr::Overflow)?
            }
        }
        Crit::K8 => cell.losing,
        Crit::K9 => i64::from(cell.m_pos != l.counted),
        Crit::K10 => as_i64(cell.trades),
        Crit::K11 => {
            if cell.trades == 0 {
                0
            } else {
                -i64::try_from(u128::from(cell.wins) * 10_000 / u128::from(cell.trades)).map_err(|_| WfErr::Overflow)?
            }
        }
        Crit::K12 => {
            let pf = if cell.loss_net == 0 {
                if cell.win_net > 0 { i64::MAX } else { 0 }
            } else {
                let x = i128::from(cell.win_net) * 1000 / -i128::from(cell.loss_net);
                i64::try_from(x).unwrap_or(i64::MAX)
            };
            -pf
        }
        Crit::K13(bar) => {
            let i = acc.bar_index(*bar).ok_or(WfErr::Overflow)?;
            let below = cell.below.get(i).copied().unwrap_or(0);
            let empty = if *bar > 0 { l.counted.saturating_sub(cell.m_traded) } else { 0 };
            as_i64(below + empty)
        }
        Crit::K14 => {
            if cell.trades == 0 {
                2
            } else {
                i64::from((l.booked & l.units) != l.units)
            }
        }
        Crit::K15(i) => host.get(usize::from(*i)).copied().unwrap_or(0),
        Crit::K16 => i64::from((l.units & !l.touched).count_ones()),
        Crit::K17 => add(l.gap, i64::from((l.units & !l.touched).count_ones()))?,
    })
}

fn pick_kind(x: &ViewInSet, v: &View, part_chosen: Option<u32>) -> PickKind {
    match x.kind {
        WinKind::Hindsight => return PickKind::Hindsight,
        WinKind::WayA => return PickKind::WayA,
        WinKind::Split | WinKind::Paper => {}
    }
    if let Some(run) = x.win_chosen {
        return PickKind::ChosenRule { run };
    }
    if let RuleProvenance::Chosen { run, .. } = v.prov
        && !cfg!(feature = "m36")
    {
        return PickKind::ChosenRule { run };
    }
    if let (Some(run), false) = (part_chosen, cfg!(feature = "m44")) {
        return PickKind::ChosenRule { run };
    }
    PickKind::Split
}
