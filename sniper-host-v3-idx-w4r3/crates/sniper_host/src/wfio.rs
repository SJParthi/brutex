//! The `common_wf` v1 wiring (LINKED: its SHA256SUMS landed 2026-11-13 17:10:25 IST): the dated session calendar, RULES and the
//! search partitions from the host's data rows, and common_wf's OWN windows, views and words files. The pick, the verdict and
//! every word come from `common_wf` (SNIPER s11 (1)); this module only reads rows and builds its inputs.

use std::path::Path;

use common_wf::rules::{PART_COLS, parse_partitions};
use common_wf::windows::{WinRow, parse_windows};
use common_wf::{CalRow, Calendar, DayKind, RunRules, Views, WindowSets, WinId};

use crate::rows::read_with_sha;

/// Everything `common_wf` reads for one context (one (timeframe, side, exit set); its partitions are the share levels).
pub struct Wf {
    /// RULES with the partitions set.
    pub rules: RunRules,
    /// common_wf's words (WORDS-STD-v1.tsv).
    pub words: common_wf::Words,
    /// The window sets.
    pub ws: WindowSets,
    /// common_wf's views (VIEWS-STD-v1.tsv).
    pub views: Views,
    /// The index of the set holding WF and TT.
    pub set: usize,
    /// (file, sha256) of each common_wf data file read.
    pub shas: Vec<(String, String)>,
}

/// The dated session table as common_wf's calendar: an entry session is a walked REGULAR day (a halt day HALTED, walked); every
/// other session (Muhurat, DR, weekend, outage, the refused days) is SPECIAL and not walked (K14: never an entry session).
///
/// # Errors
/// A calendar common_wf refuses.
pub fn calendar(sessions: &[feat_io::data::Session]) -> Result<Calendar, String> {
    let rows: Vec<CalRow> = sessions
        .iter()
        .map(|s| {
            let kind = if !s.entry {
                DayKind::Special
            } else if !s.halts.is_empty() || s.kind.contains("HALT") {
                // HOST REPAIR (with H1 (b)): common_feat repair 1 loads market-wide halts as `Session::halts` of a REGULAR
                // session (its HALT rows are no session kind any more); such a session books like any walked session (OB31).
                DayKind::Halted
            } else {
                DayKind::Regular
            };
            CalRow { day: s.day, kind, walked: s.entry, ruling: false }
        })
        .collect();
    Calendar::from_rows(&rows).map_err(|e| format!("calendar: {e}"))
}

fn kv_rows(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("key\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?.to_string(), f.next()?.to_string()))
        })
        .collect()
}

/// One search's partition row from the template file (`column  value  source`; `{tf}`, `{side}`, `{exit}`, `{level}`, `{family}`
/// filled), in common_wf's `PART_COLS` order.
#[must_use]
pub fn partition_row(template: &str, id: u32, fill: &[(&str, String)]) -> Vec<String> {
    let rows = kv_rows(template);
    PART_COLS
        .iter()
        .map(|c| {
            if *c == "id" {
                return id.to_string();
            }
            let mut v = rows.iter().find(|(k, _)| k == c).map_or_else(|| "-".to_string(), |(_, v)| v.clone());
            for (k, x) in fill {
                v = v.replace(&format!("{{{k}}}"), x);
            }
            v
        })
        .collect()
}

/// Loads RULES (the host's rows plus the declared grid of this context), the partitions, and common_wf's own files.
///
/// # Errors
/// The first refused file or row (common_wf's own error, named).
pub fn load(data: &Path, wf_data: &Path, cal: &Calendar, parts: &[Vec<String>], grid_members: u64) -> Result<Wf, String> {
    load_rules(data, "WF-RULES-SNIPER-v1.tsv", wf_data, cal, parts, grid_members)
}

/// [`load`] with a named RULES file (POSITIONAL v2: `WF-RULES-SNIPER-POS-v1.tsv`, design 6.2; the cash path reads
/// `WF-RULES-SNIPER-v1.tsv` through [`load`], byte for byte).
///
/// # Errors
/// As [`load`].
pub fn load_rules(data: &Path, rules_file: &str, wf_data: &Path, cal: &Calendar, parts: &[Vec<String>], grid_members: u64) -> Result<Wf, String> {
    let (rt, rs) = read_with_sha(&data.join(rules_file).to_string_lossy())?;
    let mut kv = kv_rows(&rt);
    // P21: the context's ONE declared SEARCH grid, LATTICE (every row from rule text; SNIPER s2 / s7, MAIN S8), its member count
    // the combos this context walks (S0 + S1 at every N), declared before the walk.
    kv.push(("grid".to_string(), format!("{}:SEARCH:LATTICE", grid_members.max(2))));
    let mut rules = RunRules::from_rows(&kv).map_err(|e| format!("RULES: {e}"))?;
    rules.partitions = parse_partitions(parts).map_err(|e| format!("partitions: {e}"))?;
    let (wt, wsha) = read_with_sha(&wf_data.join("WINDOWS-STD-v1.tsv").to_string_lossy())?;
    let rows: Vec<WinRow> = parse_windows(&wt).map_err(|e| format!("windows: {e}"))?;
    let (vt, vsha) = read_with_sha(&wf_data.join("VIEWS-STD-v1.tsv").to_string_lossy())?;
    let views = Views::from_text(&vt, &rows, &rules, &[]).map_err(|e| format!("views: {e}"))?;
    let (wd, wdsha) = read_with_sha(&wf_data.join("WORDS-STD-v1.tsv").to_string_lossy())?;
    let words = common_wf::Words::from_text(&wd).map_err(|e| format!("words: {e}"))?;
    common_wf::rules::check_paper_from(&rules, &rows, &views).map_err(|e| format!("paper_from: {e}"))?;
    let ws = common_wf::Windows::sets_from_rows(&rows, cal, rules.period).map_err(|e| format!("window sets: {e}"))?;
    let wf = rows.iter().position(|r| r.id == "WF").ok_or("no WF row")?;
    let tt = rows.iter().position(|r| r.id == "TT").ok_or("no TT row")?;
    let id = |i: usize| WinId(u8::try_from(i).unwrap_or(u8::MAX));
    let set = ws.sets.iter().position(|s| s.has(id(wf)) && s.has(id(tt))).ok_or("WF and TT are not in one window set")?;
    let shas = vec![
        (rules_file.to_string(), rs),
        ("common-wf-v1/data/WINDOWS-STD-v1.tsv".to_string(), wsha),
        ("common-wf-v1/data/VIEWS-STD-v1.tsv".to_string(), vsha),
        ("common-wf-v1/data/WORDS-STD-v1.tsv".to_string(), wdsha),
    ];
    Ok(Wf { rules, words, ws, views, set, shas })
}
