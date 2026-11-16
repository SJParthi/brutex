//! v3 (EXITS-DYNAMIC-DESIGN 2.2 as signed with the critic's R1 + R2): the fail-closed loaders of the exit MENU (`EXITS-DYN-MENU-v1.tsv`, 95
//! canonical rules) and the SETTING SETS (`EXITS-DYN-SETS-v1.tsv`, 8,181 canonical settings S1-S4, S0 = the v2 grid flagged), the rule of a
//! menu row at one event (its distance from the readings at T and the first fill p0), and a setting's rules in the first-hit TIE ORDER.
//!
//! Load rules (each refusal NAMED; a value outside its row's list is never a default):
//! 1. every rule row: a known family and kind, its levels inside the pre-registered lists, no repeated id, no repeated canonical identity
//!    (kind + parameters, family ignored: R2), every alias names that identity under another family;
//! 2. every SL row has a role SEARCH or COMPARISON (R1; the SEARCH set is Q16's: SL-P 2 / 3 / 5 % and SL-A1 0.10 / 0.25 / 0.50); other rows
//!    have none;
//! 3. every setting id parses by the grammar `XD-SL<tok>-GO<0|t>-BE<0|a|aL>-TT<0|h>-TS<0|P:a:c|R:a:c|A:a:d>-TP<0|P:j|A:k|F:F|T:H|GP:j:g|GR:j:g>-
//!    TM<0|m>`; exactly one SL; every slot resolves to one menu row (the TP slot's GP / GR through an alias when that identity is a TSL row);
//! 4. canonical settings only (R2): no trail dominated by a trail of the same kind (`dominated_trail`), no go-positive at or after a plain
//!    time exit (`go_after_time`), no breakeven-at-time at or after a plain time exit (`tt_after_time`); no repeated id or canonical key;
//! 5. the setting's role equals its SL's role (`role_mismatch`; a SEARCH id with a COMPARISON SL is refused: R1).
//!
//! The first-hit tie order (design 1, last rows): SL, TT, BE, TSL, TTP, TP, GO, TIME (the kind is a label: on a tie the fills are identical).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use common_fill::xrule::XRule;

/// The families in TIE ORDER (index = the label rank).
pub const FAMILIES: [&str; 8] = ["SL", "TT", "BE", "TSL", "TTP", "TP", "GO", "TIME"];

/// One canonical menu rule.
#[derive(Clone, Debug, Default)]
pub struct MRule {
    pub id: String,
    /// The row's family (its label rank in [`FAMILIES`]).
    pub fam: u8,
    pub kind: Kind,
    pub p1: i64,
    pub p2: i64,
    /// SL rows: true = SEARCH (eligible for THE pick), false = COMPARISON.
    pub search: bool,
    pub aliases: Vec<String>,
    pub ident: String,
    pub words: String,
}

/// The formula kinds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Kind {
    #[default]
    SlP,
    SlA1,
    SlA5,
    SlH,
    SlS,
    SlV,
    Go,
    Be,
    Tt,
    TrP,
    TrR,
    TrA,
    TpP,
    TpA,
    TpF,
    TpT,
    Time,
}

impl Kind {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "SLP" => Self::SlP,
            "SLA1" => Self::SlA1,
            "SLA5" => Self::SlA5,
            "SLH" => Self::SlH,
            "SLS" => Self::SlS,
            "SLV" => Self::SlV,
            "GO" => Self::Go,
            "BE" => Self::Be,
            "TT" => Self::Tt,
            "TRP" => Self::TrP,
            "TRR" => Self::TrR,
            "TRA" => Self::TrA,
            "TPP" => Self::TpP,
            "TPA" => Self::TpA,
            "TPF" => Self::TpF,
            "TPT" => Self::TpT,
            "TIME" => Self::Time,
            _ => return None,
        })
    }

    /// The pre-registered (p1, p2) lists of the kind (design 2.1).
    fn allowed(self) -> (&'static [i64], &'static [i64]) {
        match self {
            Self::SlP => (&[2, 3, 5, 10, 20, 50, 100], &[0]),
            Self::SlA1 => (&[10, 25, 50, 100, 200], &[0]),
            Self::SlA5 => (&[50, 100, 200, 400, 800], &[0]),
            Self::SlH => (&[0, 2, 5, 10], &[0]),
            Self::SlS => (&[5, 10, 25, 50], &[0]),
            Self::SlV => (&[50, 100, 200, 400], &[0]),
            Self::Go => (&[60, 180, 300], &[0]),
            Self::Be => (&[10, 20, 30, 50], &[0, 10]),
            Self::Tt => (&[5, 15, 30], &[0]),
            Self::TrP => (&[0, 10, 20, 30, 50, 80], &[5, 10, 20]),
            Self::TrR => (&[10, 30, 50, 80], &[10, 25, 50]),
            Self::TrA => (&[50, 100], &[50, 100]),
            Self::TpP => (&[10, 20, 30, 50, 80, 90], &[0]),
            Self::TpA => (&[50, 100, 200, 400], &[0]),
            Self::TpF => (&[50, 100, 200, 500], &[0]),
            Self::TpT => (&[15, 30, 60, 120], &[0]),
            Self::Time => (&[1, 2, 5, 15, 30, 60, 120], &[0]),
        }
    }

    /// The family a row of this kind may carry.
    fn families(self) -> &'static [&'static str] {
        match self {
            Self::SlP | Self::SlA1 | Self::SlA5 | Self::SlH | Self::SlS | Self::SlV => &["SL"],
            Self::Go => &["GO"],
            Self::Be => &["BE"],
            Self::Tt => &["TT"],
            Self::TrP | Self::TrR => &["TSL", "TTP"],
            Self::TrA => &["TSL"],
            Self::TpP | Self::TpA | Self::TpF | Self::TpT => &["TP"],
            Self::Time => &["TIME"],
        }
    }

    /// The canonical identity's kind word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::SlP => "SLP",
            Self::SlA1 => "SLA1",
            Self::SlA5 => "SLA5",
            Self::SlH => "SLH",
            Self::SlS => "SLS",
            Self::SlV => "SLV",
            Self::Go => "GO",
            Self::Be => "BE",
            Self::Tt => "TT",
            Self::TrP => "TRP",
            Self::TrR => "TRR",
            Self::TrA => "TRA",
            Self::TpP => "TPP",
            Self::TpA => "TPA",
            Self::TpF => "TPF",
            Self::TpT => "TPT",
            Self::Time => "TIME",
        }
    }

    /// A trail kind.
    #[must_use]
    pub const fn trail(self) -> bool {
        matches!(self, Self::TrP | Self::TrR | Self::TrA)
    }

    /// An SL kind.
    #[must_use]
    pub const fn sl(self) -> bool {
        matches!(self, Self::SlP | Self::SlA1 | Self::SlA5 | Self::SlH | Self::SlS | Self::SlV)
    }
}

/// The menu.
#[derive(Clone, Debug, Default)]
pub struct Menu {
    pub rules: Vec<MRule>,
    pub sha: String,
    by_ident: HashMap<String, usize>,
}

/// sha256 of a file (the RUN row's audit chain), through the cube module's reader.
#[must_use]
pub fn sha_of(p: &Path) -> String {
    crate::cube::sha256(p)
}

fn fam_rank(f: &str) -> Option<u8> {
    FAMILIES.iter().position(|x| *x == f).and_then(|i| u8::try_from(i).ok())
}

impl Menu {
    /// Loads and checks the menu.
    ///
    /// # Errors
    /// A named refusal.
    pub fn load(p: &Path) -> Result<Self, String> {
        let t = std::fs::read_to_string(p).map_err(|e| format!("menu {}: {e}", p.display()))?;
        let m = Self::parse(&t)?;
        Ok(Self { sha: sha_of(p), ..m })
    }

    /// Parses the menu text.
    ///
    /// # Errors
    /// A named refusal: `menu_head`, `menu_fields`, `menu_unknown_family`, `menu_unknown_kind`, `menu_family_kind`, `menu_level`,
    /// `menu_role`, `menu_repeated_id`, `menu_repeated_identity`, `menu_identity_text`, `menu_alias`.
    pub fn parse(t: &str) -> Result<Self, String> {
        let mut rules: Vec<MRule> = Vec::new();
        let mut ids: HashSet<String> = HashSet::new();
        let mut by_ident: HashMap<String, usize> = HashMap::new();
        let mut head = false;
        for (ln, l) in t.lines().enumerate() {
            if l.starts_with('#') || l.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = l.split('\t').collect();
            if !head {
                if f != ["rule_id", "family", "kind", "p1", "p2", "role", "aliases", "identity", "words"] {
                    return Err(format!("menu_head: line {}", ln + 1));
                }
                head = true;
                continue;
            }
            let [id, fam, kind, p1, p2, role, aliases, identity, words] = f.as_slice() else { return Err(format!("menu_fields: line {}", ln + 1)) };
            let famr = fam_rank(fam).ok_or_else(|| format!("menu_unknown_family: line {} {fam}", ln + 1))?;
            let k = Kind::parse(kind).ok_or_else(|| format!("menu_unknown_kind: line {} {kind}", ln + 1))?;
            if !k.families().contains(fam) {
                return Err(format!("menu_family_kind: line {} {fam} {kind}", ln + 1));
            }
            let (a, b): (i64, i64) = (p1.parse().map_err(|_| format!("menu_level: line {} p1 {p1}", ln + 1))?, p2.parse().map_err(|_| format!("menu_level: line {} p2 {p2}", ln + 1))?);
            let (la, lb) = k.allowed();
            if !la.contains(&a) || !lb.contains(&b) {
                return Err(format!("menu_level: line {} {kind} {a} {b} outside the pre-registered lists", ln + 1));
            }
            let search = match (k.sl(), *role) {
                (true, "SEARCH") => true,
                (true, "COMPARISON") | (false, "-") => false,
                _ => return Err(format!("menu_role: line {} {kind} role {role}", ln + 1)),
            };
            if k.sl() {
                // R1 / Q16: the SEARCH set is fixed before any data is read
                let want = matches!((k, a), (Kind::SlP, 2 | 3 | 5) | (Kind::SlA1, 10 | 25 | 50));
                if want != search {
                    return Err(format!("menu_role: line {} {kind} {a} must be {} (Q16)", ln + 1, if want { "SEARCH" } else { "COMPARISON" }));
                }
            }
            if !ids.insert((*id).to_owned()) {
                return Err(format!("menu_repeated_id: {id}"));
            }
            let ident = format!("{}:{a}:{b}", k.word());
            if *identity != ident {
                return Err(format!("menu_identity_text: line {} {identity} != {ident}", ln + 1));
            }
            if by_ident.contains_key(&ident) {
                return Err(format!("menu_repeated_identity: {id} = {ident} (R2: one row per formula)"));
            }
            let al: Vec<String> = if *aliases == "-" { Vec::new() } else { aliases.split(',').map(str::to_owned).collect() };
            for x in &al {
                // an alias names the same formula under the OTHER trail family (TTP.P.j.g / TTP.R.j.g)
                let ok = match k {
                    Kind::TrP => *x == format!("TTP.P.{a}.{b}"),
                    Kind::TrR => *x == format!("TTP.R.{a}.{b}"),
                    _ => false,
                };
                if !ok || !ids.insert(x.clone()) {
                    return Err(format!("menu_alias: {id} alias {x}"));
                }
            }
            by_ident.insert(ident.clone(), rules.len());
            rules.push(MRule { id: (*id).to_owned(), fam: famr, kind: k, p1: a, p2: b, search, aliases: al, ident, words: (*words).to_owned() });
        }
        if !head {
            return Err("menu_head: missing".to_owned());
        }
        Ok(Self { rules, sha: String::new(), by_ident })
    }

    /// The row of a canonical identity.
    #[must_use]
    pub fn of_ident(&self, ident: &str) -> Option<usize> {
        self.by_ident.get(ident).copied()
    }

    /// The number of rules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// No rule.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// The maximum rules of one setting (one per slot).
pub const MAXR: usize = 7;

/// One canonical setting.
#[derive(Clone, Debug, Default)]
pub struct Setting {
    pub id: String,
    /// The set: 1..=4 (S1..S4).
    pub set: u8,
    pub in_s0: bool,
    /// SEARCH (true) or COMPARISON.
    pub search: bool,
    /// The menu rows in TIE ORDER (label rank, then slot order).
    pub rules: Vec<u16>,
    /// Each rule's label rank (the family it plays in this setting: a TSL row in the TP slot is a TTP).
    pub labels: Vec<u8>,
    /// The SL rule (a menu row).
    pub sl: u16,
    /// The canonical key (sorted identities).
    pub key: String,
    /// S0 rows: the v2 grid's (stop k, target j (4 none), time m (4 none)).
    pub v2: Option<(usize, usize, usize)>,
    /// Families ON (SL counts 1, each other family +1): THE pick's first tie key (design 5.2).
    pub fams_on: u8,
    /// v3.3 (MAIN-RULINGS-ENGINE-v22 Addendum 28.1, search:v33): the row's role is SEARCH1D = a member of the every-year search set
    /// SEARCH-1D (`--set SEARCH1D`). A SEARCH1D setting IS a SEARCH setting (`search` true, its SL a SEARCH stop: R1), so `--set SEARCH`
    /// and every other selector, group, gate and label read it exactly as before.
    pub s1d: bool,
}

/// The sets.
#[derive(Clone, Debug, Default)]
pub struct Sets {
    pub settings: Vec<Setting>,
    pub sha: String,
    /// v3.3 (search:v33): the loaded file's name, printed in RESULTS beside its sha256 (round B printed the v1 name for any file).
    pub name: String,
}

fn slot_rule(menu: &Menu, slot: &str, tok: &str) -> Result<Option<(usize, u8)>, String> {
    if tok == "0" {
        return if slot == "SL" { Err("setting_no_sl".to_owned()) } else { Ok(None) };
    }
    let n = |s: &str| s.parse::<i64>().map_err(|_| format!("setting_token: {slot}{tok}"));
    let (ident, label) = match slot {
        "SL" => {
            let (k, v) = tok.split_once(':').ok_or_else(|| format!("setting_token: SL{tok}"))?;
            let kw = match k {
                "P" => "SLP",
                "A1" => "SLA1",
                "A5" => "SLA5",
                "H" => "SLH",
                "S" => "SLS",
                "V" => "SLV",
                _ => return Err(format!("setting_token: SL{tok}")),
            };
            (format!("{kw}:{}:0", n(v)?), 0_u8)
        }
        "GO" => (format!("GO:{}:0", n(tok)?), 6),
        "BE" => {
            let (a, lock) = tok.strip_suffix('L').map_or((tok, 0), |a| (a, 10));
            (format!("BE:{}:{lock}", n(a)?), 2)
        }
        "TT" => (format!("TT:{}:0", n(tok)?), 1),
        "TM" => (format!("TIME:{}:0", n(tok)?), 7),
        "TS" => {
            let p: Vec<&str> = tok.split(':').collect();
            let [k, a, c] = p.as_slice() else { return Err(format!("setting_token: TS{tok}")) };
            let kw = match *k {
                "P" => "TRP",
                "R" => "TRR",
                "A" => "TRA",
                _ => return Err(format!("setting_token: TS{tok}")),
            };
            (format!("{kw}:{}:{}", n(a)?, n(c)?), 3)
        }
        "TP" => {
            let p: Vec<&str> = tok.split(':').collect();
            match p.as_slice() {
                ["GP", j, g] => (format!("TRP:{}:{}", n(j)?, n(g)?), 4),
                ["GR", j, g] => (format!("TRR:{}:{}", n(j)?, n(g)?), 4),
                [k, v] => {
                    let kw = match *k {
                        "P" => "TPP",
                        "A" => "TPA",
                        "F" => "TPF",
                        "T" => "TPT",
                        _ => return Err(format!("setting_token: TP{tok}")),
                    };
                    (format!("{kw}:{}:0", n(v)?), 5)
                }
                _ => return Err(format!("setting_token: TP{tok}")),
            }
        }
        _ => return Err(format!("setting_slot: {slot}")),
    };
    let i = menu.of_ident(&ident).ok_or_else(|| format!("setting_unknown_rule: {slot}{tok} ({ident} is no menu row)"))?;
    let r = menu.rules.get(i).ok_or("menu row")?;
    // the slot must fit the row's family: SL / GO / BE / TT / TIME rows only in their own slot; TP-kind rows in TP; trails: TS takes TSL
    // rows (a TTP identity never sits in TS), TP takes TTP rows and the TSL rows whose alias names the TTP formula
    let fits = match slot {
        "SL" => r.kind.sl(),
        "GO" => r.kind == Kind::Go,
        "BE" => r.kind == Kind::Be,
        "TT" => r.kind == Kind::Tt,
        "TM" => r.kind == Kind::Time,
        "TS" => r.kind.trail() && FAMILIES.get(usize::from(r.fam)) == Some(&"TSL"),
        "TP" => matches!(r.kind, Kind::TpP | Kind::TpA | Kind::TpF | Kind::TpT) || (r.kind.trail() && (FAMILIES.get(usize::from(r.fam)) == Some(&"TTP") || !r.aliases.is_empty())),
        _ => false,
    };
    if !fits {
        return Err(format!("setting_slot_family: {slot}{tok} names {} ({})", r.id, FAMILIES.get(usize::from(r.fam)).unwrap_or(&"?")));
    }
    Ok(Some((i, label)))
}

/// Parses one setting id against the menu (load rules 3-5 except the repeated-key check, done by [`Sets::parse`]).
///
/// # Errors
/// A named refusal.
pub fn parse_setting(menu: &Menu, id: &str) -> Result<(Vec<u16>, Vec<u8>, u16, String), String> {
    let rest = id.strip_prefix("XD-").ok_or_else(|| format!("setting_grammar: {id}"))?;
    let mut toks: Vec<(&str, &str)> = Vec::new();
    // the slots in fixed order, each "-<SLOT><tok>"; tokens never contain '-' except a negative number (none in the menu)
    let parts: Vec<&str> = rest.split('-').collect();
    let order = ["SL", "GO", "BE", "TT", "TS", "TP", "TM"];
    if parts.len() != order.len() {
        return Err(format!("setting_grammar: {id} (7 slots)"));
    }
    for (p, s) in parts.iter().zip(order) {
        let tok = p.strip_prefix(s).ok_or_else(|| format!("setting_grammar: {id} slot {s}"))?;
        if tok.is_empty() {
            return Err(format!("setting_grammar: {id} empty {s}"));
        }
        toks.push((s, tok));
    }
    let mut got: Vec<(usize, u8, usize)> = Vec::new();
    for (k, (s, tok)) in toks.iter().enumerate() {
        if let Some((i, lab)) = slot_rule(menu, s, tok)? {
            got.push((i, lab, k));
        }
    }
    let sl = got.iter().find(|g| g.1 == 0).map(|g| g.0).ok_or_else(|| format!("setting_no_sl: {id}"))?;
    // R2 canonical checks
    let rule = |i: usize| menu.rules.get(i);
    for (a, _, _) in &got {
        for (b, _, _) in &got {
            if a == b {
                continue;
            }
            if let (Some(p), Some(q)) = (rule(*a), rule(*b))
                && p.kind.trail()
                && p.kind == q.kind
                && q.p1 <= p.p1
                && q.p2 <= p.p2
            {
                return Err(format!("dominated_trail: {id} ({} never fires before {})", p.id, q.id));
            }
        }
    }
    if got.len() != got.iter().map(|g| g.0).collect::<HashSet<_>>().len() {
        return Err(format!("setting_repeated_rule: {id}"));
    }
    let tm = got.iter().filter_map(|g| rule(g.0)).find(|r| r.kind == Kind::Time).map(|r| r.p1);
    if let Some(m) = tm {
        if got.iter().filter_map(|g| rule(g.0)).any(|r| r.kind == Kind::Go && 60 * m <= r.p1) {
            return Err(format!("go_after_time: {id}"));
        }
        if got.iter().filter_map(|g| rule(g.0)).any(|r| r.kind == Kind::Tt && m <= r.p1) {
            return Err(format!("tt_after_time: {id}"));
        }
    }
    got.sort_by_key(|g| (g.1, g.2));
    let mut key: Vec<String> = got.iter().filter_map(|g| rule(g.0)).map(|r| r.ident.clone()).collect();
    key.sort();
    let rules: Vec<u16> = got.iter().map(|g| u16::try_from(g.0).unwrap_or(u16::MAX)).collect();
    let labels: Vec<u8> = got.iter().map(|g| g.1).collect();
    Ok((rules, labels, u16::try_from(sl).unwrap_or(u16::MAX), key.join("|")))
}

impl Sets {
    /// Loads and checks the sets against the menu.
    ///
    /// # Errors
    /// A named refusal.
    pub fn load(p: &Path, menu: &Menu) -> Result<Self, String> {
        let t = std::fs::read_to_string(p).map_err(|e| format!("sets {}: {e}", p.display()))?;
        let s = Self::parse(&t, menu)?;
        Ok(Self { sha: sha_of(p), name: p.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()), ..s })
    }

    /// Parses the sets text.
    ///
    /// # Errors
    /// A named refusal (`sets_head`, `sets_fields`, `sets_set`, `sets_s0`, `role_mismatch`, `repeated_setting_id`, `duplicate_setting`,
    /// `sets_canonical_text`, every [`parse_setting`] refusal).
    pub fn parse(t: &str, menu: &Menu) -> Result<Self, String> {
        let mut settings = Vec::new();
        let mut ids: HashSet<String> = HashSet::new();
        let mut keys: HashSet<String> = HashSet::new();
        let mut head = false;
        for (ln, l) in t.lines().enumerate() {
            if l.starts_with('#') || l.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = l.split('\t').collect();
            if !head {
                if f != ["setting_id", "set", "in_s0", "role", "canonical"] {
                    return Err(format!("sets_head: line {}", ln + 1));
                }
                head = true;
                continue;
            }
            let [id, set, s0, role, canonical] = f.as_slice() else { return Err(format!("sets_fields: line {}", ln + 1)) };
            let setn = match *set {
                "S1" => 1,
                "S2" => 2,
                "S3" => 3,
                "S4" => 4,
                _ => return Err(format!("sets_set: line {} {set}", ln + 1)),
            };
            let in_s0 = match *s0 {
                "1" => true,
                "0" => false,
                _ => return Err(format!("sets_s0: line {}", ln + 1)),
            };
            let (rules, labels, sl, key) = parse_setting(menu, id)?;
            if key != *canonical {
                return Err(format!("sets_canonical_text: {id}: {canonical} != {key}"));
            }
            let slr = menu.rules.get(usize::from(sl)).ok_or("sl row")?;
            // v3.3 (Addendum 28.1): SEARCH1D = a SEARCH setting of the every-year set SEARCH-1D (the R1 check below holds for it too)
            let (search, s1d) = match *role {
                "SEARCH" => (true, false),
                "SEARCH1D" => (true, true),
                "COMPARISON" => (false, false),
                _ => return Err(format!("role_mismatch: {id} role {role}")),
            };
            if search != slr.search {
                return Err(format!("role_mismatch: {id} is {role} but its SL {} is {} (R1)", slr.id, if slr.search { "SEARCH" } else { "COMPARISON" }));
            }
            if !ids.insert((*id).to_owned()) {
                return Err(format!("repeated_setting_id: {id}"));
            }
            if !keys.insert(key.clone()) {
                return Err(format!("duplicate_setting: {id} ({key} listed before)"));
            }
            // the v2 grid coordinates of an S0 row (stop k, target j, time m)
            let v2 = if in_s0 {
                if setn != 1 {
                    return Err(format!("sets_s0: {id} is S0 but not S1"));
                }
                let k = crate::trade::STOP_PCT.iter().position(|&x| x == slr.p1 && slr.kind == Kind::SlP).ok_or_else(|| format!("sets_s0: {id} stop"))?;
                let mut j = 4;
                let mut m = 4;
                for &r in &rules {
                    let x = menu.rules.get(usize::from(r)).ok_or("row")?;
                    match x.kind {
                        Kind::TpP => j = crate::trade::TGT_PCT.iter().position(|&g| g == x.p1).ok_or_else(|| format!("sets_s0: {id} target"))?,
                        Kind::Time => m = crate::trade::TIME_MIN.iter().position(|&g| g == x.p1).ok_or_else(|| format!("sets_s0: {id} time"))?,
                        Kind::SlP => {}
                        _ => return Err(format!("sets_s0: {id} has a rule outside the v2 grid")),
                    }
                }
                Some((k, j, m))
            } else {
                None
            };
            let fams_on = u8::try_from(rules.len()).unwrap_or(u8::MAX);
            settings.push(Setting { id: (*id).to_owned(), set: setn, in_s0, search, rules, labels, sl, key, v2, fams_on, s1d });
        }
        if !head {
            return Err("sets_head: missing".to_owned());
        }
        Ok(Self { settings, sha: String::new(), name: String::new() })
    }

    /// The settings of a set selector ("S0", "S1".."S4", "S1-S3", "S1-S4", "S1+S3", "ALL", "SEARCH", v3.3 "SEARCH1D").
    ///
    /// # Errors
    /// An unknown selector.
    pub fn select(&self, sel: &str) -> Result<Vec<usize>, String> {
        // v3.3 (Addendum 28.1, search:v33): the every-year search set SEARCH-1D = the rows whose role is SEARCH1D, in file order
        // MUTANT m_v33_s1d_select_search: the SEARCH1D selector returns every SEARCH setting (the trim lost)
        if sel == "SEARCH1D" {
            let keep = |s: &Setting| if cfg!(feature = "m_v33_s1d_select_search") { s.search } else { s.s1d };
            return Ok((0..self.settings.len()).filter(|&i| self.settings.get(i).is_some_and(keep)).collect());
        }
        let want: Vec<u8> = match sel {
            "S0" => return Ok((0..self.settings.len()).filter(|&i| self.settings.get(i).is_some_and(|s| s.in_s0)).collect()),
            "S1" => vec![1],
            "S2" => vec![2],
            "S3" => vec![3],
            "S4" => vec![4],
            "S1+S3" => vec![1, 3],
            "S1-S3" => vec![1, 2, 3],
            "S1-S4" | "ALL" => vec![1, 2, 3, 4],
            // v3.2 (addendum 6 / 15: the headline search space = SEARCH stops only): every set's SEARCH settings
            "SEARCH" => return Ok((0..self.settings.len()).filter(|&i| self.settings.get(i).is_some_and(|s| s.search)).collect()),
            o => return Err(format!("--set S0|S1|S2|S3|S4|S1+S3|S1-S3|S1-S4|SEARCH|SEARCH1D, not {o}")),
        };
        Ok((0..self.settings.len()).filter(|&i| self.settings.get(i).is_some_and(|s| want.contains(&s.set))).collect())
    }
}

/// The readings at T of one event (design 2, no look-ahead) as the menu needs them. `None` = unready.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rd {
    /// Wilder ATR 14 of the option's own 1-minute bars ended by T, paisa x 1,000 (floored at each step).
    pub atr1: Option<i64>,
    /// Wilder ATR 14 of the 5-second candles (range only) completed by T, paisa x 1,000.
    pub atr5: Option<i64>,
    /// The highest HIGH of the event window's present 1-minute bars (paisa; 0 = none).
    pub hwin: i64,
    /// close_T - start (the screener's gain numerator, paisa).
    pub rise: i64,
    /// (sum of (HIGH - LOW)^2, traded seconds m) over [T - 300 s, T); None when m < 30.
    pub v300: Option<(u128, u64)>,
    /// (time value at T, trading minutes from T to 15:30 of the expiry day); Err = the refusal code.
    pub theta: Result<(i64, i64), &'static str>,
}

impl Default for Rd {
    fn default() -> Self {
        Self { atr1: None, atr5: None, hwin: 0, rise: 0, v300: None, theta: Err("theta_none") }
    }
}

/// Why a rule is ABSENT at an event (counted per rule; an absent SL refuses the event for its settings: REQ 47).
pub const ABSENT_WHY: [&str; 7] = ["atr1_unready", "atr5_unready", "vol_unready", "theta_none", "theta_no_calendar", "tpf_not_below_entry", "theta_level_le_zero"];

/// The rule of menu row `r` at an event: Ok((rule, the distance floor of 1 paisa bound)) or Err(the absence code index in [`ABSENT_WHY`]).
/// `p0` = the share level's first fill price.
///
/// # Errors
/// The absence index.
pub fn rule_at(r: &MRule, rd: &Rd, p0: i64) -> Result<(XRule, bool), u8> {
    let pct_ceil = |k: i64| (p0 * k + 99) / 100;
    let pct_floor = |k: i64| p0 * k / 100;
    let f1 = |d: i64| (d.max(1), d < 1);
    let mul_ceil = |k_h: i64, x1000: i64| -> i64 { let num = i128::from(k_h) * i128::from(x1000); let den = 100_000_i128; i64::try_from((num + den - 1).div_euclid(den)).unwrap_or(i64::MAX) };
    let mul_floor = |k_h: i64, x1000: i64| -> i64 { i64::try_from((i128::from(k_h) * i128::from(x1000)).div_euclid(100_000)).unwrap_or(i64::MAX) };
    Ok(match r.kind {
        Kind::SlP => {
            let (d, b) = f1(pct_ceil(r.p1));
            (XRule::Stop { x: d }, b)
        }
        Kind::SlA1 => {
            let a = rd.atr1.ok_or(0_u8)?;
            let (d, b) = f1(mul_ceil(r.p1, a));
            (XRule::Stop { x: d }, b)
        }
        Kind::SlA5 => {
            let a = rd.atr5.ok_or(1_u8)?;
            let (d, b) = f1(mul_ceil(r.p1, a));
            (XRule::Stop { x: d }, b)
        }
        Kind::SlH => (XRule::StopAt { level: rd.hwin.saturating_add(5 * r.p1) }, false),
        Kind::SlS => {
            let num = i128::from(r.p1) * i128::from(rd.rise.max(0));
            let (d, b) = f1(i64::try_from((num + 99).div_euclid(100)).unwrap_or(i64::MAX));
            (XRule::Stop { x: d }, b)
        }
        Kind::SlV => {
            let (s, m) = rd.v300.ok_or(2_u8)?;
            // the smallest d with 16 m d^2 >= (4k)^2 x 60 x S, 4k = p1 / 25 (k in hundredths: 50 -> 2, 100 -> 4, 200 -> 8, 400 -> 16)
            let fk = u128::try_from(r.p1 / 25).unwrap_or(0);
            let num = fk * fk * 60 * s;
            let den = 16 * u128::from(m.max(1));
            let mut d = isqrt(num / den);
            while d * d * den < num {
                d += 1;
            }
            let (d, b) = f1(i64::try_from(d).unwrap_or(i64::MAX));
            (XRule::Stop { x: d }, b)
        }
        Kind::Go => (XRule::GoPositive { secs: u32::try_from(r.p1).unwrap_or(0) }, false),
        Kind::Be => (XRule::ArmedStop { act: pct_floor(r.p1), off: -pct_floor(r.p2) }, false),
        Kind::Tt => (XRule::TimedStop { after: u32::try_from(r.p1 * 60).unwrap_or(0), off: 0 }, false),
        Kind::TrP | Kind::TrR if cfg!(feature = "m_sf3_ttp_close") && FAMILIES.get(usize::from(r.fam)) == Some(&"TTP") => {
            // MUTANT m_sf3_ttp_close: the trailing take-profit closes at its activation touch (a plain target)
            (XRule::Target { x: pct_floor(r.p1).max(1) }, false)
        }
        Kind::TrP => {
            let (d, b) = f1(pct_floor(r.p2));
            (XRule::Trail { act: pct_floor(r.p1), dist: d }, b)
        }
        Kind::TrR => (XRule::TrailPct { act: pct_floor(r.p1), bp: u32::try_from(r.p2 * 100).unwrap_or(0) }, false),
        Kind::TrA => {
            let a = rd.atr1.ok_or(0_u8)?;
            let (d, b) = f1(mul_floor(r.p2, a));
            (XRule::Trail { act: mul_floor(r.p1, a), dist: d }, b)
        }
        Kind::TpP => {
            let (x, b) = f1(pct_floor(r.p1));
            (XRule::Target { x }, b)
        }
        Kind::TpA => {
            let a = rd.atr1.ok_or(0_u8)?;
            let (x, b) = f1(mul_floor(r.p1, a));
            (XRule::Target { x }, b)
        }
        Kind::TpF => {
            // MUTANT m_sf3_tpf_armed_above: the floor armed at or above the entry
            if r.p1 >= p0 && !cfg!(feature = "m_sf3_tpf_armed_above") {
                return Err(5);
            }
            (XRule::TargetAt { level: r.p1 }, false)
        }
        Kind::TpT => {
            let (tv, mt) = rd.theta.map_err(|w| if w == "theta_no_calendar" { 4_u8 } else { 3_u8 })?;
            let raw = i64::try_from(i128::from(tv) * i128::from(r.p1) / i128::from(mt.max(1))).unwrap_or(i64::MAX);
            let (x, b) = f1(raw);
            if x >= p0 {
                // R(s) <= p0: the level R(s) - x is at or below 0 for every s: it never fires (counted)
                return Err(6);
            }
            (XRule::Target { x }, b)
        }
        Kind::Time => (XRule::TimeExit { secs: u32::try_from(r.p1 * 60).unwrap_or(0) }, false),
    })
}

/// Integer square root (floor).
#[must_use]
pub fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Does a rule depend on the event window (its timeframe): SL-H (the spike's high) and SL-S (the spike's rise).
#[must_use]
pub const fn tf_dependent(k: Kind) -> bool {
    matches!(k, Kind::SlH | Kind::SlS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> (String, String) {
        let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        (std::fs::read_to_string(d.join("EXITS-DYN-MENU-v1.tsv")).unwrap_or_default(), std::fs::read_to_string(d.join("EXITS-DYN-SETS-v1.tsv")).unwrap_or_default())
    }

    #[test]
    fn sf3_menu_and_sets_load_with_the_signed_counts() {
        let (m, s) = files();
        let menu = Menu::parse(&m).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(menu.len(), 95, "98 rules minus the 3 TTP rows identical to TSL rows (R2)");
        let sets = Sets::parse(&s, &menu).unwrap_or_else(|e| panic!("{e}"));
        let n = |k: u8| sets.settings.iter().filter(|x| x.set == k).count();
        assert_eq!((n(1), n(2), n(3), n(4)), (392, 1_232, 1_537, 5_020));
        assert_eq!(sets.settings.len(), 8_181);
        assert_eq!(sets.settings.iter().filter(|x| x.in_s0).count(), 125);
        // R1: SEARCH settings are exactly those whose SL is SL-P 2/3/5 or SL-A1 0.10/0.25/0.50
        for x in &sets.settings {
            let sl = &menu.rules[usize::from(x.sl)];
            assert_eq!(x.search, sl.search);
        }
        // the S0 rows map 1:1 onto the v2 grid's 125 (k, j, m)
        let mut v2: Vec<(usize, usize, usize)> = sets.settings.iter().filter_map(|x| x.v2).collect();
        v2.sort_unstable();
        v2.dedup();
        assert_eq!(v2.len(), 125);
    }

    #[test]
    fn sf3_menu_fail_closed() {
        let (m, s) = files();
        let menu = Menu::parse(&m).unwrap_or_else(|e| panic!("{e}"));
        // a level outside its list, an unknown kind, a repeated identity, a wrong role: each a named refusal
        let bad_level = m.replacen("SL.P.2\tSL\tSLP\t2\t", "SL.P.2\tSL\tSLP\t4\t", 1);
        assert!(Menu::parse(&bad_level).err().is_some_and(|e| e.starts_with("menu_level") || e.starts_with("menu_identity_text")));
        let bad_kind = m.replacen("\tSLP\t2\t", "\tSLQ\t2\t", 1);
        assert!(Menu::parse(&bad_kind).err().is_some_and(|e| e.starts_with("menu_unknown_kind")));
        let dup = format!("{m}TTP.P.30.5\tTTP\tTRP\t30\t5\t-\t-\tTRP:30:5\tdup\n");
        assert!(Menu::parse(&dup).err().is_some_and(|e| e.starts_with("menu_repeated")));
        let bad_role = m.replacen("SL.P.10\tSL\tSLP\t10\t0\tCOMPARISON", "SL.P.10\tSL\tSLP\t10\t0\tSEARCH", 1);
        assert!(Menu::parse(&bad_role).err().is_some_and(|e| e.starts_with("menu_role")));
        // settings: a repeated id, a dominated trail, a GO after TIME, a SEARCH id with a COMPARISON SL, an unknown token, no SL
        let line = s.lines().find(|l| l.starts_with("XD-SLP:2-GO0")).unwrap_or_default();
        let rep = format!("{s}{line}\n");
        assert!(Sets::parse(&rep, &menu).err().is_some_and(|e| e.starts_with("repeated_setting_id")));
        assert!(parse_setting(&menu, "XD-SLP:5-GO0-BE0-TT0-TSP:10:10-TPGP:50:10-TM0").err().is_some_and(|e| e.starts_with("dominated_trail")));
        assert!(parse_setting(&menu, "XD-SLP:5-GO300-BE0-TT0-TS0-TP0-TM5").err().is_some_and(|e| e.starts_with("go_after_time")));
        assert!(parse_setting(&menu, "XD-SLP:5-GO0-BE0-TT15-TS0-TP0-TM15").err().is_some_and(|e| e.starts_with("tt_after_time")));
        assert!(parse_setting(&menu, "XD-SLP:7-GO0-BE0-TT0-TS0-TP0-TM0").err().is_some_and(|e| e.starts_with("setting_unknown_rule")));
        assert!(parse_setting(&menu, "XD-SL0-GO0-BE0-TT0-TS0-TP0-TM0").err().is_some_and(|e| e.starts_with("setting_no_sl")));
        assert!(parse_setting(&menu, "XD-SLP:5-GO0-BE0-TT0-TSP:50:5-TP0-TM0").err().is_some_and(|e| e.starts_with("setting_slot_family")));
        let srch = s.replacen("XD-SLP:10-GO0-BE0-TT0-TS0-TP0-TM0\tS1\t1\tCOMPARISON", "XD-SLP:10-GO0-BE0-TT0-TS0-TP0-TM0\tS1\t1\tSEARCH", 1);
        assert!(Sets::parse(&srch, &menu).err().is_some_and(|e| e.starts_with("role_mismatch")));
        // the TP slot's GP:30:5 resolves (through the alias) to the TSL row TS.P.30.5, labelled TTP
        let (r, lab, _, _) = parse_setting(&menu, "XD-SLP:2-GO0-BE0-TT0-TSP:0:10-TPGP:30:5-TM0").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r.len(), 3);
        assert_eq!(lab, vec![0, 3, 4]);
        assert_eq!(menu.rules[usize::from(r[2])].id, "TS.P.30.5");
    }

    #[test]
    fn sf3_distance_floor_counted() {
        let (m, _) = files();
        let menu = Menu::parse(&m).unwrap_or_else(|e| panic!("{e}"));
        let row = |id: &str| menu.rules.iter().find(|r| r.id == id).cloned().unwrap_or_default();
        let rd = Rd { atr1: Some(3_000), atr5: Some(1_000), hwin: 900, rise: 400, v300: Some((100, 50)), theta: Ok((60, 375)) };
        // SL 2% of Rs 0.05 = 1 paisa (ceil): not a floor; TP 10% of 5 paisa = 0 -> 1 paisa: the floor binds (counted)
        assert_eq!(rule_at(&row("SL.P.2"), &rd, 5), Ok((XRule::Stop { x: 1 }, false)));
        assert_eq!(rule_at(&row("TP.P.10"), &rd, 5), Ok((XRule::Target { x: 1 }, true)));
        // TP 30% at p0 >= 5 never floors (critic F2)
        assert_eq!(rule_at(&row("TP.P.30"), &rd, 5), Ok((XRule::Target { x: 1 }, false)));
        // SL-A1 0.10 x ATR1 3.000 paisa = 0.3 -> ceil 1; SL-V k 1 on S 100, m 50: d^2 >= 60 x 100 / 50 = 120 -> 11
        assert_eq!(rule_at(&row("SL.A1.10"), &rd, 100), Ok((XRule::Stop { x: 1 }, false)));
        assert_eq!(rule_at(&row("SL.V.100"), &rd, 100), Ok((XRule::Stop { x: 11 }, false)));
        // SL-H 2 ticks above the window high (absolute); SL-S 0.25 x rise 400 = 100
        assert_eq!(rule_at(&row("SL.H.2"), &rd, 100), Ok((XRule::StopAt { level: 910 }, false)));
        assert_eq!(rule_at(&row("SL.S.25"), &rd, 100), Ok((XRule::Stop { x: 100 }, false)));
        // unready readings: absent with their codes
        let un = Rd { atr1: None, atr5: None, v300: None, theta: Err("theta_none"), ..rd };
        assert_eq!(rule_at(&row("SL.A1.10"), &un, 100), Err(0));
        assert_eq!(rule_at(&row("SL.A5.50"), &un, 100), Err(1));
        assert_eq!(rule_at(&row("SL.V.50"), &un, 100), Err(2));
        assert_eq!(rule_at(&row("TP.T.15"), &un, 100), Err(3));
        // BE 30% with a 10% lock; TT 15 min; TTP relative 25%
        assert_eq!(rule_at(&row("BE.30L"), &rd, 1_000), Ok((XRule::ArmedStop { act: 300, off: -100 }, false)));
        assert_eq!(rule_at(&row("TT.15"), &rd, 1_000), Ok((XRule::TimedStop { after: 900, off: 0 }, false)));
        assert_eq!(rule_at(&row("TTP.R.50.25"), &rd, 1_000), Ok((XRule::TrailPct { act: 500, bp: 2_500 }, false)));
    }

    #[test]
    fn sf3_tp_floor_not_armed_at_or_above_entry() {
        let (m, _) = files();
        let menu = Menu::parse(&m).unwrap_or_else(|e| panic!("{e}"));
        let row = menu.rules.iter().find(|r| r.id == "TP.F.100").cloned().unwrap_or_default();
        let rd = Rd { theta: Err("theta_none"), ..Rd::default() };
        assert_eq!(rule_at(&row, &rd, 100), Err(5), "F = p0: absent (tpf_not_below_entry)");
        assert_eq!(rule_at(&row, &rd, 99), Err(5), "F above p0: absent");
        assert_eq!(rule_at(&row, &rd, 101), Ok((XRule::TargetAt { level: 100 }, false)));
    }

    #[test]
    fn sf3_tp_theta_level_and_absence() {
        let (m, _) = files();
        let menu = Menu::parse(&m).unwrap_or_else(|e| panic!("{e}"));
        let row = menu.rules.iter().find(|r| r.id == "TP.T.60").cloned().unwrap_or_default();
        // TV Rs 3.75 over 375 minutes: theta 1 paisa a minute: 60 minutes = 60 paisa
        assert_eq!(rule_at(&row, &Rd { theta: Ok((375, 375)), ..Rd::default() }, 1_000), Ok((XRule::Target { x: 60 }, false)));
        // a level at or below zero never fires: absent, counted
        assert_eq!(rule_at(&row, &Rd { theta: Ok((375, 375)), ..Rd::default() }, 60), Err(6));
        assert_eq!(rule_at(&row, &Rd { theta: Err("theta_no_calendar"), ..Rd::default() }, 1_000), Err(4));
    }
}
