//! The RULES values the module reads (4.5), the declared grids with their SIGNED provenance (P21, BX1, A1: the core reads the
//! signature, never infers it), the partitions with a provenance per value of every partition dimension (A2) and the side setting
//! as part of the search only where the family has a side dial (A4), and the `paper_from` validation (Z2, F3).

use crate::date::parse_mk;
use crate::views::Views;
use crate::windows::{WinKind, WinRow};
use crate::{Code, Mk, ReadSet, WfErr};

/// A signed provenance of a rule, a views file version (header or row), a window row or a partition value (Z1, A2, A3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleProvenance {
    /// Written from rule text, its own split verdicts unshown when fixed (or ruled so knowingly by MAIN's dated line, D56).
    RuledFromText,
    /// Chosen after a Brutex run showed split verdicts on months these windows' test parts hold: W27 on its picks.
    Chosen {
        /// The months shown.
        shown: ReadSet,
        /// The run that showed them (W27's `{run}`).
        run: u32,
    },
    /// MAIN's A2 alternative: LATTICE knowingly, a value chosen from the operator's chart of `chart`; W14K beside every verdict.
    RuledKnowingly {
        /// The chart's month.
        chart: Mk,
    },
}

/// Parses `RULED_FROM_TEXT [quote]` | `CHOSEN(YYYY-MM..YYYY-MM; run)` | `RULED_KNOWINGLY(YYYY-MM)`.
#[must_use]
pub fn parse_prov(s: &str) -> Option<RuleProvenance> {
    if s == "RULED_FROM_TEXT" || s.starts_with("RULED_FROM_TEXT ") {
        return Some(RuleProvenance::RuledFromText);
    }
    if let Some(x) = s.strip_prefix("CHOSEN(").and_then(|x| x.strip_suffix(')')) {
        let (range, run) = x.split_once("; ")?;
        let (a, b) = range.split_once("..")?;
        let (a, b) = (parse_mk(a)?, parse_mk(b)?);
        if a > b || run.is_empty() || !run.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        return Some(RuleProvenance::Chosen { shown: ReadSet { from: a, to: b }, run: run.parse().ok()? });
    }
    if let Some(x) = s.strip_prefix("RULED_KNOWINGLY(").and_then(|x| x.strip_suffix(')')) {
        return Some(RuleProvenance::RuledKnowingly { chart: parse_mk(x)? });
    }
    None
}

/// A declared grid's signed provenance (P21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// Every combination of dials at levels written from rule text: the EMPTY read set.
    Lattice,
    /// A member or level picked from an earlier run's results or a chart: that run's months.
    Chosen {
        /// The months read.
        set: ReadSet,
        /// The run.
        run: u32,
    },
}
/// A declared grid's role (P13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridRole {
    /// Offered to views.
    Search,
    /// Walked and reported, never offered (W26).
    Fixed,
}
/// A grid declared in RULES before the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredGrid {
    /// Its member count.
    pub members: u64,
    /// Its role.
    pub role: GridRole,
    /// Its signed provenance.
    pub provenance: Provenance,
    /// MAIN's line for a 1-member LATTICE grid.
    pub main_line: Option<String>,
}
impl DeclaredGrid {
    /// The grid's resolved read set: EMPTY for LATTICE, its months for CHOSEN (never EMPTY: m27).
    #[must_use]
    pub const fn read_set(&self) -> ReadSet {
        match self.provenance {
            Provenance::Lattice => ReadSet::EMPTY,
            Provenance::Chosen { set, .. } => set,
        }
    }
}
/// A FIXED code of a FIXED grid (P13), named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedCode {
    /// The combo code.
    pub code: Code,
    /// Its FIXED grid.
    pub grid: u8,
    /// Its name (OP-A, ...).
    pub name: String,
}
/// The run's trade order (P6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeOrder {
    /// The host's order inside a session, unchecked.
    Arrival,
    /// Non-decreasing (close_s, net), checked on arrival (R4 (d)).
    SeqLossesFirst,
}
/// The hole source (P7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoleSource {
    /// No hole flag: the second reading is not available.
    None,
    /// The lane's flag.
    Lane,
    /// The fill crate's `hole_span`.
    FillCrate,
}
/// Session-end marks (R3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MtmMarks {
    /// No marks (every intraday lane).
    None,
    /// The host feeds a mark at every walked session end with an open position.
    SessionEnd,
}
/// A partition's role (P22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartRole {
    /// A real partition: one SEARCH.
    Real,
    /// A comparison partition: figures only (R4 (c)).
    Comparison,
}

/// The W9 columns of a partition row, in order; the `prov_*` columns are A2's per-value provenance.
pub const PART_COLS: &[&str] = &["id", "role", "world", "what", "reading", "underlying", "signal_source", "instrument", "size_rule", "account", "universe", "rank_set", "share_level", "exit_set", "timeframe_family", "side_dial", "side_setting", "fill_law", "fill_level", "timing", "family", "timeframe", "period", "proof_step", "data_labels", "prov_account", "prov_universe", "prov_share_level", "prov_exit_set", "prov_timeframe_family", "prov_side_setting", "prov_fill_level", "prov_timing", "prov_reading"];
/// The partition dimensions that carry a provenance (A2 item 1), in order.
pub const PROV_DIMS: &[&str] = &["account", "universe", "share_level", "exit_set", "timeframe_family", "side_setting", "fill_level", "timing", "reading"];

/// One partition (W9): one SEARCH in the real world (MAIN 14:31:36 IST; A4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartitionDef {
    /// The id.
    pub id: u32,
    /// The role.
    pub role: PartRole,
    /// Every column by name, as written.
    pub cols: Vec<(String, String)>,
    /// Does the family have a side dial (A4)? Without one, a both-sides account is ONE search.
    pub side_dial: bool,
    /// A2: each dimension value's signed provenance (absent = not a declared value).
    pub prov: Vec<(String, RuleProvenance)>,
}
impl PartitionDef {
    /// A column's value ("-" when absent).
    #[must_use]
    pub fn get(&self, k: &str) -> &str {
        self.cols.iter().find(|(a, _)| a == k).map_or("-", |(_, b)| b.as_str())
    }
    /// The world label ("real" for the real world).
    #[must_use]
    pub fn world(&self) -> &str {
        self.get("world")
    }
    /// The first CHOSEN value's run, in PROV_DIMS order (A2: W27 on its picks).
    #[must_use]
    pub fn chosen_run(&self) -> Option<u32> {
        PROV_DIMS.iter().find_map(|d| self.prov.iter().find(|(k, _)| k == d).and_then(|(_, p)| if let RuleProvenance::Chosen { run, .. } = p { Some(*run) } else { None }))
    }
    /// A2, fail closed (repair of check-build-v1 B2): every value of a REAL partition that is not a COMPARISON partition carries
    /// a declared provenance: its own `prov_<dim>`, MAIN's standing ruling ([`ADANIENT_ALONE_CHART`]), or the run's signed
    /// RULES `partition_provenance_default` (RULED_FROM_TEXT with its signed line). A value of "-" is no value. Nothing is ever
    /// read as RULED_FROM_TEXT because a host left it out.
    ///
    /// # Errors
    /// `NoPartitionProvenance` naming this partition and the first undeclared dimension in `PROV_DIMS` order.
    pub fn declared(&self, r: &RunRules) -> Result<(), WfErr> {
        if r.is_comparison(self) || r.partition_prov_default.is_some() || cfg!(feature = "m47") {
            return Ok(());
        }
        for d in PROV_DIMS {
            // m53 (the checker's mutant Y5): the universe dimension skipped
            if cfg!(feature = "m53") && *d == "universe" {
                continue;
            }
            if self.get(d) != "-" && !self.prov.iter().any(|(k, _)| k == d) {
                return Err(WfErr::NoPartitionProvenance { partition: self.id, dim: d });
            }
        }
        Ok(())
    }
}

/// MAIN's standing A2 ruling (MAIN-SIGNED-COMMON-WF-20261113.md A2, signed 2026-11-13 15:36:02 IST; SNIPER-REQUIREMENTS s12,
/// 15:36:14 IST): REQ 76's ADANIENT-alone universe is LATTICE knowingly, "the instrument and its pattern were chosen from the
/// operator's chart of 2026-04 (a test month of both windows)". The core applies it itself (W14K beside every verdict of an
/// ADANIENT-alone partition), so no host can leave the label out; a declaration that differs is refused (`RulingContradicted`).
pub const ADANIENT_ALONE_CHART: &str = "2026-04";

/// Is a partition an ADANIENT-alone search? Its universe is "ADANIENT" (or "ADANIENT alone"), or it names no universe and its
/// underlying is ADANIENT ("ADANIENT", "ADANIENT spot", ...). ASCII case is ignored.
#[must_use]
pub fn adanient_alone(universe: &str, underlying: &str) -> bool {
    let first_is = |s: &str| s.split_whitespace().next().is_some_and(|x| x.eq_ignore_ascii_case("ADANIENT"));
    if universe == "-" {
        first_is(underlying)
    } else {
        first_is(universe) && universe.split_whitespace().skip(1).all(|x| x.eq_ignore_ascii_case("alone"))
    }
}

/// Parses partition rows (each in `PART_COLS` order).
///
/// MAIN's A2 ruling of an ADANIENT-alone partition is applied here when the row leaves `prov_universe` out ("-"). Whether every
/// other value is declared is checked against RULES by [`PartitionDef::declared`] ([`check_partitions`], the load door).
///
/// # Errors
/// `BadRow(P, n, rule)`: 1 a role or side dial word, 2 a side setting the family's dial forbids (A4: per-side partitions of a
/// both-sides account, m45), 3 a bad or duplicate id, 4 a provenance that does not parse; `RulingContradicted` a declared
/// universe provenance of an ADANIENT-alone partition other than MAIN's RULED_KNOWINGLY(2026-04).
pub fn parse_partitions(rows: &[Vec<String>]) -> Result<Vec<PartitionDef>, WfErr> {
    let mut out: Vec<PartitionDef> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let bad = |rule: u8| WfErr::BadRow { file: b'P', row: n, rule };
        let col = |k: &str| -> &str { PART_COLS.iter().position(|c| *c == k).and_then(|j| r.get(j)).map_or("-", String::as_str) };
        let ids = col("id");
        if ids.is_empty() || !ids.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad(3));
        }
        let id: u32 = ids.parse().map_err(|_| bad(3))?;
        if out.iter().any(|p| p.id == id) {
            return Err(bad(3));
        }
        let role = match col("role") {
            "REAL" => PartRole::Real,
            "COMPARISON" => PartRole::Comparison,
            _ => return Err(bad(1)),
        };
        let side_dial = match col("side_dial") {
            "yes" => true,
            "no" => false,
            _ => return Err(bad(1)),
        };
        let side = col("side_setting");
        if (!cfg!(feature = "m45") && !side_dial && !matches!(side, "both" | "-")) || (side_dial && !matches!(side, "long only" | "short only" | "both")) {
            return Err(bad(2));
        }
        let mut prov = Vec::new();
        for d in PROV_DIMS {
            let v = col(&format!("prov_{d}"));
            if v == "-" {
                continue;
            }
            prov.push(((*d).to_owned(), parse_prov(v).ok_or_else(|| bad(4))?));
        }
        if adanient_alone(col("universe"), col("underlying")) && !cfg!(feature = "m48") {
            let ruled = RuleProvenance::RuledKnowingly { chart: parse_mk(ADANIENT_ALONE_CHART).ok_or(WfErr::BadDay)? };
            match prov.iter().find(|(k, _)| k == "universe") {
                None => prov.push(("universe".to_owned(), ruled)),
                Some((_, x)) if *x == ruled => {}
                Some(_) => return Err(WfErr::RulingContradicted { partition: id, dim: "universe" }),
            }
        }
        let cols = PART_COLS.iter().enumerate().map(|(j, c)| ((*c).to_owned(), r.get(j).cloned().unwrap_or_else(|| "-".to_owned()))).collect();
        out.push(PartitionDef { id, role, cols, side_dial, prov });
    }
    Ok(out)
}

/// The RULES values the module reads (4.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRules {
    /// The run's first and last month (P3).
    pub period: (Mk, Mk),
    /// The RULES signing month (Z2).
    pub signed: Mk,
    /// The trade order (P6).
    pub trade_order: TradeOrder,
    /// The hole source (P7).
    pub hole_source: HoleSource,
    /// The base capital in paisa (bp bars, BUCKET).
    pub base_capital: i64,
    /// Session-end marks (R3).
    pub mtm_marks: MtmMarks,
    /// The first month a PAPER reading may read (Z2, F3).
    pub paper_from: Mk,
    /// Every grid declared before the run (P21).
    pub grids: Vec<DeclaredGrid>,
    /// The FIXED codes (P13), named.
    pub fixed: Vec<FixedCode>,
    /// Partition ids of role COMPARISON (R4 (c)).
    pub comparison: Vec<u32>,
    /// The headline partition, or none (D39).
    pub headline_partition: Option<u32>,
    /// W01's proof step (F15).
    pub proof: String,
    /// W01's data labels (F15).
    pub data: String,
    /// The run id.
    pub run: u32,
    /// LATTICE grids hold levels the operator named from his charts (W14's extension, D50 default).
    pub charts: bool,
    /// The run holds carried trades (W19's open-position clause).
    pub carried: bool,
    /// REQ 74's replayed L levels, as printed (Q3).
    pub replay_levels: Vec<String>,
    /// The partitions (W9), set by the host before the walk.
    pub partitions: Vec<PartitionDef>,
    /// The signed default provenance of every partition value no `prov_<dim>` declares (repair of check-build-v1 B2, A2): RULES
    /// `partition_provenance_default=RULED_FROM_TEXT <signed line>`, the line kept. None: every value must be declared itself.
    pub partition_prov_default: Option<String>,
    /// The identity hash of the RULES rows.
    pub ident: u64,
}

impl RunRules {
    /// Parses RULES (key, value) rows.
    ///
    /// # Errors
    /// `BadRow(R, 0, 1)` an unknown key, `BadRow(R, 0, 2)` a bad or missing value (a `partition_provenance_default` other than
    /// RULED_FROM_TEXT with a signed line included), `OneMemberLattice` (P21).
    pub fn from_rows(kv: &[(String, String)]) -> Result<Self, WfErr> {
        let bad = |rule: u8| WfErr::BadRow { file: b'R', row: 0, rule };
        let mut r = Self {
            period: (0, -1),
            signed: 0,
            trade_order: TradeOrder::SeqLossesFirst,
            hole_source: HoleSource::Lane,
            base_capital: 200_000_000,
            mtm_marks: MtmMarks::None,
            paper_from: 0,
            grids: Vec::new(),
            fixed: Vec::new(),
            comparison: Vec::new(),
            headline_partition: None,
            proof: "-".to_owned(),
            data: "-".to_owned(),
            run: 1,
            charts: false,
            carried: false,
            replay_levels: Vec::new(),
            partitions: Vec::new(),
            partition_prov_default: None,
            ident: 0x0123_4567,
        };
        let (mut has_period, mut has_signed, mut has_paper) = (false, false, false);
        for (k, v) in kv {
            r.ident = crate::mix(r.ident ^ crate::text_hash(k) ^ crate::text_hash(v).rotate_left(17));
            match k.as_str() {
                "period" => {
                    let (a, b) = v.split_once("..").ok_or_else(|| bad(2))?;
                    r.period = (parse_mk(a).ok_or_else(|| bad(2))?, parse_mk(b).ok_or_else(|| bad(2))?);
                    has_period = true;
                }
                "signed" => {
                    r.signed = parse_mk(v).ok_or_else(|| bad(2))?;
                    has_signed = true;
                }
                "order" => r.trade_order = match v.as_str() {
                    "ARRIVAL" => TradeOrder::Arrival,
                    "SEQ_LOSSES_FIRST" => TradeOrder::SeqLossesFirst,
                    _ => return Err(bad(2)),
                },
                "holes" => r.hole_source = match v.as_str() {
                    "none" => HoleSource::None,
                    "lane" => HoleSource::Lane,
                    "fill-crate" => HoleSource::FillCrate,
                    _ => return Err(bad(2)),
                },
                "base" => r.base_capital = v.parse().map_err(|_| bad(2))?,
                "marks" => r.mtm_marks = match v.as_str() {
                    "none" => MtmMarks::None,
                    "session_end" => MtmMarks::SessionEnd,
                    _ => return Err(bad(2)),
                },
                "paper_from" => {
                    r.paper_from = parse_mk(v).ok_or_else(|| bad(2))?;
                    has_paper = true;
                }
                "grid" => {
                    let c: Vec<&str> = v.splitn(4, ':').collect();
                    let members: u64 = c.first().and_then(|x| x.parse().ok()).ok_or_else(|| bad(2))?;
                    let role = match c.get(1).copied() {
                        Some("SEARCH") => GridRole::Search,
                        Some("FIXED") => GridRole::Fixed,
                        _ => return Err(bad(2)),
                    };
                    let provenance = match c.get(2).copied() {
                        Some("LATTICE") => Provenance::Lattice,
                        Some(p) => match parse_prov(p) {
                            Some(RuleProvenance::Chosen { shown, run }) => Provenance::Chosen { set: shown, run },
                            _ => return Err(bad(2)),
                        },
                        None => return Err(bad(2)),
                    };
                    let main_line = c.get(3).filter(|x| **x != "-").map(|x| (*x).to_owned());
                    if members == 1 && provenance == Provenance::Lattice && main_line.is_none() && !cfg!(feature = "m28") {
                        return Err(WfErr::OneMemberLattice);
                    }
                    r.grids.push(DeclaredGrid { members, role, provenance, main_line });
                }
                "fixed" => {
                    let (g, list) = v.split_once(':').ok_or_else(|| bad(2))?;
                    let grid: u8 = g.parse().map_err(|_| bad(2))?;
                    for e in list.split(',') {
                        let (name, code) = e.split_once('=').ok_or_else(|| bad(2))?;
                        r.fixed.push(FixedCode { code: Code([code.parse().map_err(|_| bad(2))?, 0, 0, 0]), grid, name: name.to_owned() });
                    }
                }
                "comparison" => {
                    for p in v.split(',') {
                        r.comparison.push(p.parse().map_err(|_| bad(2))?);
                    }
                }
                "headline_partition" => r.headline_partition = if v == "none" { None } else { Some(v.parse().map_err(|_| bad(2))?) },
                "proof" => r.proof.clone_from(v),
                "data" => r.data.clone_from(v),
                "run" => r.run = v.parse().map_err(|_| bad(2))?,
                "charts" => r.charts = v == "yes",
                "carried" => r.carried = v == "yes",
                "replay" => r.replay_levels = v.split(',').map(str::to_owned).collect(),
                // only RULED_FROM_TEXT with its signed line: a CHOSEN or RULED_KNOWINGLY value is a fact of ONE value, never a default
                "partition_provenance_default" => {
                    let line = v.strip_prefix("RULED_FROM_TEXT ").filter(|x| !x.trim().is_empty()).ok_or_else(|| bad(2))?;
                    r.partition_prov_default = Some(line.to_owned());
                }
                _ => return Err(bad(1)),
            }
        }
        if !(has_period && has_signed && has_paper) {
            return Err(bad(2));
        }
        Ok(r)
    }
    /// The partition `id` (W9).
    ///
    /// # Errors
    /// `NoPartitionRow` (Y4, OB54).
    pub fn partition(&self, id: u32) -> Result<&PartitionDef, WfErr> {
        self.partitions.iter().find(|p| p.id == id).ok_or(WfErr::NoPartitionRow)
    }
    /// Is partition `id` a COMPARISON partition (its role, or named in RULES)?
    #[must_use]
    pub fn is_comparison(&self, p: &PartitionDef) -> bool {
        p.role == PartRole::Comparison || self.comparison.contains(&p.id)
    }
}

/// A2, fail closed (repair of check-build-v1 B2): every partition of the run declares the provenance of each of its values
/// ([`PartitionDef::declared`]). [`check_paper_from`] runs it first, so the load door every host already calls refuses the run.
///
/// # Errors
/// `NoPartitionProvenance` (the first partition and dimension in file and `PROV_DIMS` order).
pub fn check_partitions(r: &RunRules) -> Result<(), WfErr> {
    for p in &r.partitions {
        p.declared(r)?;
    }
    Ok(())
}

/// Z2 / F3: `paper_from` must be later than the signing month and than every CHOSEN read set's `to` (grids, the views header
/// and rows, the window rows, the partition values) and every chart source month (A2's RULED_KNOWINGLY), and never before a
/// PAPER row's first month.
///
/// It first runs [`check_partitions`] (B2: a partition value with no declared provenance refuses the run at load, by name).
///
/// # Errors
/// `NoPartitionProvenance`, `PaperFromSeen`.
pub fn check_paper_from(r: &RunRules, rows: &[WinRow], v: &Views) -> Result<(), WfErr> {
    check_partitions(r)?;
    let pf = r.paper_from;
    if pf <= r.signed {
        return Err(WfErr::PaperFromSeen);
    }
    let mut seen: Vec<Mk> = Vec::new();
    let mut push = |p: &RuleProvenance| match p {
        RuleProvenance::Chosen { shown, .. } => seen.push(shown.to),
        RuleProvenance::RuledKnowingly { chart } => seen.push(*chart),
        RuleProvenance::RuledFromText => {}
    };
    for g in &r.grids {
        if let Provenance::Chosen { set, run } = g.provenance {
            push(&RuleProvenance::Chosen { shown: set, run });
        }
    }
    push(&v.header);
    for x in &v.views {
        if let Some(p) = &x.own_prov {
            push(p);
        }
    }
    for w in rows {
        if let Some(p) = &w.prov {
            push(p);
        }
    }
    for p in &r.partitions {
        for (_, x) in &p.prov {
            push(x);
        }
    }
    if seen.iter().any(|t| pf <= *t) {
        return Err(WfErr::PaperFromSeen);
    }
    if rows.iter().any(|w| w.kind == WinKind::Paper && w.test_from.is_some_and(|t| pf < t)) {
        return Err(WfErr::PaperFromSeen);
    }
    Ok(())
}
