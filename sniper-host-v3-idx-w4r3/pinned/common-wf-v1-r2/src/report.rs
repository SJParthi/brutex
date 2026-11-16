//! Every printed line (section 7; report time, loops allowed: T-SCAN's named exemptions): a window row's reading of a pick (W01-W11,
//! W13, W21-W24, W26, W27), the pick label (W19), ties (W16), the margin (W17), the closest misses (P17), the re-used holdout (W15)
//! with A3's `fixed_after` words, A2's W14K line, the luck line (P23, W18) and the per-SEARCH method line (P19; MAIN 14:31:36 IST:
//! "ONE search = (account or universe, share level, exit set) [and for later families also the timeframe family and the side]";
//! A4; F1: W12 names its search, views and ids; F6: W12b says "outside every honest row's learning months").

use crate::acc::{Acc, Facts, Trade};
use crate::date::{mk_of_day, mk_str, overlap_str, range_str, years_str};
use crate::rules::{GridRole, HoleSource, MtmMarks, PROV_DIMS, PartitionDef, Provenance, RuleProvenance, RunRules};
use crate::verdict::{NotRun, PickKind, PickWin, Reading, Verdict};
use crate::views::{Code, Counts, Crit, Elig, Kept, Role, View, Views};
use crate::windows::{NotRunWhy, WinKind, WinRow, WindowSets, Windows};
use crate::words::{Words, fill_template, fmt_pct, fmt_rs, share_bp, sum};
use crate::{Mk, NO_GRID, Partition, WfErr, WinId, sub};

fn item(w: &Windows, u: u8, f: &Facts) -> String {
    let label = w.tab.units.get(usize::from(u)).map_or("", |x| x.label.as_str());
    let bit = 1u64 << u;
    let c = f.units.get(usize::from(u)).copied().unwrap_or(crate::acc::UnitCell::EMPTY);
    if f.nodata & bit != 0 {
        format!("{label} no data for this instrument")
    } else if f.missing & bit != 0 {
        format!("{label} no session in the data")
    } else if c.trades == 0 {
        format!("{label} no trade")
    } else {
        format!("{label} {}", fmt_rs(c.net))
    }
}
fn items(w: &Windows, list: &[u8], f: &Facts) -> String {
    list.iter().map(|u| item(w, *u, f)).collect::<Vec<_>>().join("; ")
}
/// Every test unit of `v` positive as booked and, where a hole source exists, in the second reading.
#[must_use]
pub fn all_pos(w: &Windows, v: WinId, f: &Facts, r: &RunRules) -> bool {
    let Some(s) = w.spec(v) else { return false };
    (s.test_units & !f.booked) == 0 && (r.hole_source == HoleSource::None || (s.test_units & !f.second) == 0)
}
fn second_words(w: &Windows, list: &[u8], f: &Facts, r: &RunRules) -> Result<String, WfErr> {
    if r.hole_source == HoleSource::None {
        return Ok("not available (this lane marks no feed holes)".to_owned());
    }
    let mut d = Vec::new();
    for u in list {
        let c = f.units.get(usize::from(*u)).copied().unwrap_or(crate::acc::UnitCell::EMPTY);
        if c.hole_win != 0 {
            let label = w.tab.units.get(usize::from(*u)).map_or("", |x| x.label.as_str());
            d.push(format!("{label} {}", fmt_rs(sub(c.net, c.hole_win)?)));
        }
    }
    Ok(if d.is_empty() { "the same".to_owned() } else { d.join("; ") })
}
/// W26's provenance words of a grid.
#[must_use]
pub fn prov_words(r: &RunRules, grid: u8) -> String {
    match r.grids.get(usize::from(grid)).map(|g| g.provenance) {
        Some(Provenance::Chosen { set, .. }) => format!("chosen knowing {}, HINDSIGHT", range_str(set.from, set.to)),
        _ => "a grid written from rule text".to_owned(),
    }
}
/// The FIXED-overlap note on a FIXED subject's figures line (BX2 (2a)).
#[must_use]
pub fn fixed_overlap_note(r: &RunRules, grid: u8) -> String {
    format!("; also a member of grid g{grid} ({}); any pick of it is that grid's, shown in its view rows", prov_words(r, grid))
}

/// The lines of window row `v`'s reading of `pick` (P11): a verdict line and its figures, or figures only.
///
/// # Errors
/// `PaperSeen` (Z2), `ProofMissing` (F15), `NoPartitionProvenance` (A2, B2: the partition's values are not all declared),
/// `SetMismatch`, `Overflow`.
#[allow(clippy::too_many_arguments, reason = "the report context: windows, row, reading, pick, facts, re-walk, rules, partition")]
pub fn reading_lines(words: &Words, w: &Windows, v: WinId, rv: &(Reading, Verdict), pick: &PickWin, pick_row: &WinRow, f: &Facts, acc: &Acc, r: &RunRules, part: &PartitionDef, fixed_note: &str, level_facts: Option<&Facts>) -> Result<Vec<String>, WfErr> {
    part.declared(r)?;
    let row = w.row(v).ok_or(WfErr::SetMismatch)?;
    let s = w.spec(v).ok_or(WfErr::SetMismatch)?;
    let t = row.test_name.as_str();
    let mut out = Vec::new();
    let figs = |extra: &str| -> Result<String, WfErr> { words.fill("W10", &[("T", t), ("list", &items(w, &s.test_list, f)), ("second", &second_words(w, &s.test_list, f, r)?), ("fixed_note", extra)]) };
    match rv.0 {
        Reading::Comparison => {
            if row.kind == WinKind::Split
                && let Some((a, b)) = s.test
            {
                out.push(words.fill("W11", &[("what", part.get("what")), ("years", &years_str(a, b)), ("list", &items(w, &s.test_list, f))])?);
            }
            return Ok(out);
        }
        Reading::World => {
            if row.kind == WinKind::Split {
                let yn = if all_pos(w, v, f, r) { "yes" } else { "no" };
                out.push(words.fill("W23", &[("world", part.world()), ("what", part.get("what")), ("code", &pick.code.0[0].to_string()), ("list", &items(w, &s.test_list, f)), ("all_pos", yn)])?);
            }
            return Ok(out);
        }
        _ => {}
    }
    match row.kind {
        WinKind::WayA => {
            let (a, b) = s.test.ok_or(WfErr::SetMismatch)?;
            let inside = if pick.learn.0 <= a && b <= pick.learn.1 { " (inside this pick's training months)" } else { "" };
            out.push(words.fill("W22", &[("id", &row.id), ("test_years", &years_str(a, b)), ("inside", inside), ("list", &items(w, &s.test_list, f))])?);
            return Ok(out);
        }
        WinKind::Paper => {
            let (a, b) = s.test.ok_or(WfErr::SetMismatch)?;
            if a < r.paper_from {
                return Err(WfErr::PaperSeen);
            }
            let list: Vec<u8> = s.test_list.iter().copied().filter(|u| w.tab.units.get(usize::from(*u)).is_some_and(|x| x.from >= r.paper_from)).collect();
            out.push(words.fill("W21", &[("paper_from", &mk_str(r.paper_from)), ("to", &mk_str(b)), ("list", &items(w, &list, f))])?);
            return Ok(out);
        }
        WinKind::Hindsight => {
            if matches!(pick.kind, PickKind::Hindsight | PickKind::ChosenRule { .. }) {
                out.push(words.fill("W13", &[("period", &range_str(r.period.0, r.period.1))])?);
            }
            return Ok(out);
        }
        WinKind::Split => {}
    }
    match rv.1 {
        Verdict::Passed { booked_only } => {
            if r.proof.is_empty() || r.proof == "-" || r.data.is_empty() || r.data == "-" {
                return Err(WfErr::ProofMissing);
            }
            out.push(words.fill("W01", &[("T", t), ("booked_only", if booked_only { "booked reading only; " } else { "" }), ("proof", &r.proof), ("data", &r.data)])?);
        }
        Verdict::NotPassedYears { bad, at_level, .. } => {
            let src = level_facts.unwrap_or(f);
            let at = at_level.and_then(|l| r.replay_levels.get(usize::from(l)));
            let list = s.test_list.iter().filter(|u| bad.0 & (1u64 << **u) != 0).map(|u| { let x = item(w, *u, src); at.map_or_else(|| x.clone(), |l| format!("{x} at L {l} s")) }).collect::<Vec<_>>().join("; ");
            out.push(words.fill("W02", &[("T", t), ("list", &list)])?);
        }
        Verdict::NotPassedHole { bad2 } => {
            let mut xs = Vec::new();
            for u in s.test_list.iter().filter(|u| bad2.0 & (1u64 << **u) != 0) {
                let c = f.units.get(usize::from(*u)).copied().unwrap_or(crate::acc::UnitCell::EMPTY);
                let label = w.tab.units.get(usize::from(*u)).map_or("", |x| x.label.as_str());
                xs.push(format!("{label} {} without its {} winning hole trades", fmt_rs(sub(c.net, c.hole_win)?), c.hole_wins));
            }
            out.push(words.fill("W03", &[("T", t), ("list", &xs.join("; "))])?);
        }
        Verdict::NotRun(n) => {
            let note = match n {
                NotRun::OtherProtocol { overlap } => {
                    let owner = fill_template(&pick_row.owner_words, &[("learn", &range_str(pick.learn.0, pick.learn.1))])?;
                    out.push(words.fill("W04", &[("T", t), ("pick_protocol", &pick_row.protocol), ("owner", &owner), ("overlap", &overlap_str(overlap.0, overlap.1))])?);
                    ""
                }
                NotRun::Hindsight => {
                    out.push(words.fill("W06", &[("T", t)])?);
                    ""
                }
                NotRun::WayA => {
                    out.push(words.fill("W09", &[("T", t)])?);
                    ""
                }
                NotRun::ChosenRule { run } => {
                    out.push(words.fill("W27", &[("T", t), ("run", &run.to_string())])?);
                    ""
                }
                NotRun::Fixed { grid } => {
                    out.push(words.fill("W26", &[("T", t), ("what", "a fixed combo"), ("provenance", &prov_words(r, grid))])?);
                    fixed_note
                }
                NotRun::NoPick | NotRun::NoLearnMonth | NotRun::NoTestMonth | NotRun::SetTooBig => "",
            };
            out.push(figs(note)?);
            return Ok(out);
        }
        Verdict::Figures => {
            let (a, b) = s.test.ok_or(WfErr::SetMismatch)?;
            let yn = if all_pos(w, v, f, r) { "yes" } else { "no" };
            out.push(words.fill("W05", &[("T", t), ("pick_protocol", &pick_row.protocol), ("learn", &range_str(pick.learn.0, pick.learn.1)), ("test_years", &years_str(a, b)), ("figures", &items(w, &s.test_list, f)), ("all_pos", yn)])?);
            return Ok(out);
        }
    }
    let (m, mm, worst) = acc.test_months(v)?;
    out.push(words.fill("W24", &[("T", t), ("m", &m.to_string()), ("M", &mm.to_string()), ("rs", &worst.map_or_else(|| "none".to_owned(), fmt_rs))])?);
    out.push(figs("")?);
    Ok(out)
}

/// The positions open at a learning end (W19; counted from the report-time re-walk): entered on or before it, closed after it.
#[must_use]
pub fn open_at_learn_end(trades: &[Trade], learn_to: Mk) -> u64 {
    trades.iter().filter(|t| mk_of_day(t.day) <= learn_to && mk_of_day(t.exit_day) > learn_to).fold(0, |n, _| n + 1)
}

/// W19, the pick label.
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn pick_label(words: &Words, row: &WinRow, learn: (Mk, Mk), k: &Kept, n_open: u64, r: &RunRules) -> Result<String, WfErr> {
    let marks = r.mtm_marks == MtmMarks::SessionEnd;
    let carried_run = r.carried || marks;
    let carried = if carried_run && row.learn.is_some() {
        let mark = if marks {
            if n_open == 0 { fmt_rs(0) } else { k.learn.mark_end.map_or_else(|| fmt_rs(0), fmt_rs) }
        } else {
            "no marks fed".to_owned()
        };
        format!("; positions open at the learning end: {n_open}, marked {mark}")
    } else {
        String::new()
    };
    let dd = if marks {
        "with session-end marks"
    } else if carried_run {
        "closed-trade drawdown only"
    } else {
        "closed trades; a dip inside a trade shows in the run-down only"
    };
    let mut twin = String::new();
    if let Some(fx) = r.fixed.iter().find(|x| x.code == k.key.code)
        && let Some(t) = k.tickets.kept().iter().find(|t| t.grid != NO_GRID)
    {
        twin = format!(" (also pre-registered as fixed code {}; this pick comes from the g{} search, {})", fx.name, t.grid, prov_words(r, t.grid));
    }
    let lr = if row.learn.is_some() { range_str(learn.0, learn.1) } else { "-".to_owned() };
    words.fill("W19", &[("pick_label", &row.pick_label), ("learn", &lr), ("n_learn_trades", &k.learn.trades.to_string()), ("n_learn_sessions", &k.learn.sessions.to_string()), ("carried", &carried), ("dd_basis", dd), ("fixed_twin", &twin)])
}

/// W16.
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn ties_line(words: &Words, ties: u64) -> Result<String, WfErr> {
    if ties > 1 { words.fill("W16n", &[("n", &ties.to_string())]) } else { words.fill("W16", &[]) }
}

fn crit_fmt(c: &Crit, v: i64) -> String {
    if c.money() { fmt_rs(v) } else { v.to_string() }
}
/// W17 / W17b, the margin (P18): the first criterion on which #1 differs from the next DIFFERENT key.
///
/// # Errors
/// `BadRow(T, 0, 9)`, `Overflow`.
pub fn margin_line(words: &Words, kept: &[Kept], view: &View) -> Result<String, WfErr> {
    let Some(a) = kept.first() else { return words.fill("W17b", &[("k", "0")]) };
    let used = |k: &Kept| -> Vec<i64> { k.key.parts.get(..usize::from(k.key.n)).unwrap_or(&[]).to_vec() };
    let ap = used(a);
    let Some(b) = kept.iter().find(|x| used(x) != ap) else { return words.fill("W17b", &[("k", &kept.len().to_string())]) };
    let bp = used(b);
    let i = ap.iter().zip(bp.iter()).position(|(x, y)| x != y).unwrap_or(0);
    let (Some(c), Some(x), Some(y)) = (view.key.get(i), ap.get(i), bp.get(i)) else { return words.fill("W17b", &[("k", &kept.len().to_string())]) };
    words.fill("W17", &[("criterion", c.name()), ("v1", &crit_fmt(c, *x)), ("v2", &crit_fmt(c, *y)), ("g", &crit_fmt(c, sub(*y, *x)?))])
}

/// W15, the re-used holdout line of a split pick (Z2: "the first months no rule of this run was chosen with").
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn holdout_line(words: &Words, row: &WinRow, r: &RunRules) -> Result<String, WfErr> {
    let test = match (row.test_from, row.test_to) {
        (Some(a), Some(b)) => range_str(a, b),
        _ => "-".to_owned(),
    };
    words.fill("W15", &[("test", &test), ("paper_from", &mk_str(r.paper_from))])
}

/// A2's W14K lines: one per partition value signed RULED_KNOWINGLY (MAIN's ADANIENT choice), beside every verdict.
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn knowingly_lines(words: &Words, part: &PartitionDef, honest: &[&WinRow]) -> Result<Vec<String>, WfErr> {
    let mut out = Vec::new();
    for d in PROV_DIMS {
        let Some((_, RuleProvenance::RuledKnowingly { chart })) = part.prov.iter().find(|(k, _)| k == d) else { continue };
        let hold: Vec<&&WinRow> = honest.iter().filter(|h| h.test_from.is_some_and(|a| a <= *chart) && h.test_to.is_some_and(|b| *chart <= b)).collect();
        let cw = if hold.len() == honest.len() {
            if honest.len() == 2 { "both windows".to_owned() } else { format!("all {} windows", honest.len()) }
        } else if hold.is_empty() {
            "no window".to_owned()
        } else {
            hold.iter().map(|h| h.id.as_str()).collect::<Vec<_>>().join(", ")
        };
        out.push(words.fill("W14K", &[("chart_month", &mk_str(*chart)), ("chart_windows", &cw), ("instrument", part.get("underlying"))])?);
    }
    Ok(out)
}

fn rule_words(v: &View) -> String {
    v.elig
        .iter()
        .filter_map(|e| match e {
            Elig::E6 => None,
            Elig::E1 => Some("at least 1 learning trade".to_owned()),
            Elig::E2 => Some("every learning year positive".to_owned()),
            Elig::E3 => Some("at most one losing learning year".to_owned()),
            Elig::E4(n) => Some(format!("at least {n} learning trades in every learning year")),
            Elig::E5(n) => Some(format!("at least {n} learning trades a session")),
            Elig::E7 => Some("not every learning year positive".to_owned()),
        })
        .collect::<Vec<_>>()
        .join(" and ")
}
/// W07 (no pick) in its standard, comparison (W07c) or world (W07w) form (Y7).
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn no_pick_line(words: &Words, v: &View, row: &WinRow, c: &Counts, part: &PartitionDef, r: &RunRules) -> Result<String, WfErr> {
    let rw = if c.offered == 0 {
        "no combo was offered".to_owned()
    } else if c.fixed_offered == c.offered {
        "every offer came through a FIXED grid ticket".to_owned()
    } else if c.rejected_chosen_grid > 0 && c.rejected_chosen_grid + c.fixed_offered == c.offered {
        "every offer is from a CHOSEN grid".to_owned()
    } else if c.rejected_read_set + c.rejected_no_grid + c.rejected_chosen_grid + c.fixed_offered == c.offered {
        "every offer was shaped by months outside the learning part".to_owned()
    } else {
        rule_words(v)
    };
    if r.is_comparison(part) {
        words.fill("W07c", &[("what", part.get("what")), ("rule_words", &rw)])
    } else if part.world() != "real" {
        words.fill("W07w", &[("world", part.world()), ("what", part.get("what")), ("rule_words", &rw)])
    } else {
        words.fill("W07", &[("T", &row.test_name), ("rule_words", &rw)])
    }
}

/// W08: the window is not in this run (P3).
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn not_run_line(words: &Words, row: &WinRow, why: NotRunWhy) -> Result<String, WfErr> {
    let t = row.test_name.as_str();
    match why {
        NotRunWhy::SetTooBig => words.fill("W08s", &[("T", t)]),
        NotRunWhy::Learning | NotRunWhy::Test => {
            let learn = row.learn.map_or_else(|| "-".to_owned(), |(a, b)| range_str(a, b));
            let owner = fill_template(&row.owner_words, &[("learn", &learn)])?;
            words.fill("W08", &[("T", t), ("part_kind", if why == NotRunWhy::Learning { "learning" } else { "test" }), ("owner", &owner)])
        }
    }
}

/// The closest-miss checks of a MISS entry (P17): every failing learning unit with its gap.
#[must_use]
pub fn miss_checks(w: &Windows, v: WinId, f: &Facts) -> Vec<String> {
    let Some(s) = w.spec(v) else { return Vec::new() };
    s.learn_list
        .iter()
        .filter(|u| f.booked & (1u64 << **u) == 0)
        .map(|u| {
            let label = w.tab.units.get(usize::from(*u)).map_or("", |x| x.label.as_str());
            let c = f.units.get(usize::from(*u)).copied().unwrap_or(crate::acc::UnitCell::EMPTY);
            if c.trades == 0 {
                format!("{label} no trade (gap: a trade and Rs 0.01)")
            } else {
                format!("{label} {} (gap: {})", fmt_rs(c.net), fmt_rs(1i64.saturating_sub(c.net)))
            }
        })
        .collect()
}

/// One honest row's item in the method line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemWord {
    /// Its own verdict PASSED.
    Passed,
    /// Its own verdict NOT PASSED.
    NotPassed,
    /// Its rule is CHOSEN (W27): NOT PASSED with the W27 item (A2).
    Chosen(u32),
    /// No pick (W07).
    NoPick,
}
/// One honest row's HEADLINE pick in the method line. Its window row and its partition are part of the item (repair of
/// check-build-v1 B1): `method_line` takes exactly one item per honest row in use, every one of the line's own partition, each
/// named by its row's HEADLINE view (repair 2, check-build-v1-r2 B1-R).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodItem {
    /// The honest window row whose HEADLINE view picked it (WF, TT).
    pub win: WinId,
    /// The partition (the SEARCH) the pick belongs to.
    pub partition: Partition,
    /// The view id (WF-R73, TT-R73).
    pub view: String,
    /// The pick label.
    pub label: String,
    /// The code, if a pick exists.
    pub code: Option<Code>,
    /// Its own verdict.
    pub word: ItemWord,
}
/// The search words of a partition (F1, MAIN 14:31:36 IST, A4: the side setting only where the family has a side dial).
#[must_use]
pub fn search_words(p: &PartitionDef, r: &RunRules) -> String {
    let rank = if p.get("rank_set") == "-" { String::new() } else { format!(" (rank set {})", p.get("rank_set")) };
    let side = if p.side_dial { format!("; side {}", p.get("side_setting")) } else { String::new() };
    format!(
        "search: {} {}, {}; account {}; universe {}{rank}; share level {}; exit set {}; timeframe family {}{side}; period {}; run {}",
        p.get("underlying"),
        p.get("instrument"),
        p.get("size_rule"),
        p.get("account"),
        p.get("universe"),
        p.get("share_level"),
        p.get("exit_set"),
        p.get("timeframe_family"),
        p.get("period"),
        r.run
    )
}
/// The HEADLINE view of window row `w`, if the views hold exactly one (`Views::from_text` refuses a split row without exactly one).
fn headline_of(views: &Views, w: WinId) -> Option<&View> {
    let mut it = views.views.iter().filter(|v| v.win == w && v.role == Role::Headline);
    match (it.next(), it.next()) {
        (Some(v), None) => Some(v),
        _ => None,
    }
}
/// The run's window sets name every honest split row of its file exactly once: in ONE set (in use) or once among the rows not
/// run, and `ids` (`ws.honest()`) are exactly the ones in use, each once (repair 2, B1-R: a host's hand-built `WindowSets` with a
/// set doubled or dropped is refused, never read as fewer or more honest tests).
fn honest_rows_whole(ws: &WindowSets, ids: &[WinId]) -> bool {
    if ids.windows(2).any(|p| matches!(p, [a, b] if a == b)) {
        return false;
    }
    let file = ws.rows.iter().filter(|x| x.kind == WinKind::Split && x.honest);
    let mut n_file = 0usize;
    for row in file {
        n_file += 1;
        let w = WinId(row.idx);
        let in_sets = ws.sets.iter().filter(|s| s.has(w)).count();
        let not_run = ws.not_run.iter().filter(|(x, _)| *x == w).count();
        if in_sets + not_run != 1 || (in_sets == 1) != ids.contains(&w) {
            return false;
        }
    }
    n_file > 0 && ids.iter().all(|h| ws.rows.get(usize::from(h.0)).is_some_and(|x| x.idx == h.0 && x.kind == WinKind::Split && x.honest))
}
/// The method line of ONE search (P19): W12 and, with two or more honest rows, W12b. None for a COMPARISON partition or a
/// non-real world (Y7). Never across searches (REQ 73d; MAIN 14:31:36 IST: "Both honest tests PASSED" only when the SAME
/// search has its walk-forward pick PASSED and its train/test pick PASSED; else each word apart and "same combo: yes | no").
///
/// The honest rows are the RUN's own (repair 2, check-build-v1-r2 B1-R; plan P19 and plan:831, repair 3 U8: the method line takes
/// every set's windows): `ws.honest()`, the honest split rows of EVERY window set in use, read here and never taken from a caller.
/// The items are exactly one item per honest row in use (repair of check-build-v1 B1), every one of `part`, each its row's
/// HEADLINE view's pick (an item of a BESIDE, MISS or any other view is refused); they print in the honest rows' file order
/// whatever order the caller gives. With NO honest row in use (a period that holds no honest row wholly; R1) the line is
/// "{Both | All n} honest tests NOT PASSED" naming every honest row of the file "NOT PASSED (not run)", never a refusal.
///
/// # Errors
/// `NoPartitionProvenance` (A2, B2), `SearchMismatch` an item of another partition, `MethodItems` (window sets that do not name
/// every honest row of the file exactly once, an item of a row that is not an honest row in use, an item whose view is not its
/// row's HEADLINE view, a row with no item or two, a word but "no pick" without a pick code or "no pick" with one),
/// `BadRow(T, 0, 9)`.
pub fn method_line(words: &Words, given: &[MethodItem], ws: &WindowSets, views: &Views, part: &PartitionDef, r: &RunRules) -> Result<Option<Vec<String>>, WfErr> {
    if r.is_comparison(part) || part.world() != "real" {
        return Ok(None);
    }
    part.declared(r)?;
    let ids = ws.honest();
    let items: Vec<&MethodItem> = if cfg!(feature = "m46") {
        given.iter().collect()
    } else {
        if given.iter().any(|x| x.partition != Partition(part.id)) {
            return Err(WfErr::SearchMismatch);
        }
        if !cfg!(feature = "m51") && !honest_rows_whole(ws, &ids) {
            return Err(WfErr::MethodItems);
        }
        // m52 (the checker's mutant Y3): an extra item of a row not in use silently dropped
        let given: Vec<&MethodItem> = given.iter().filter(|x| !cfg!(feature = "m52") || ids.contains(&x.win)).collect();
        for x in &given {
            if !ids.contains(&x.win) {
                return Err(WfErr::MethodItems);
            }
            if !cfg!(feature = "m50") && headline_of(views, x.win).is_none_or(|v| v.id != x.view) {
                return Err(WfErr::MethodItems);
            }
        }
        let mut ordered = Vec::with_capacity(ids.len());
        for h in &ids {
            let mut of_row = given.iter().filter(|x| x.win == *h);
            match (of_row.next(), of_row.next()) {
                (Some(x), None) => ordered.push(*x),
                _ => return Err(WfErr::MethodItems),
            }
        }
        if ordered.iter().any(|x| (x.word == ItemWord::NoPick) != x.code.is_none()) {
            return Err(WfErr::MethodItems);
        }
        ordered
    };
    let honest_rows: Vec<&WinRow> = ids.iter().map(|h| ws.row(*h)).collect();
    let honest: &[&WinRow] = &honest_rows;
    if honest.is_empty() {
        // R1: no honest row in use: NOT PASSED, every honest row of the file named as not run (its W08 line says why)
        let file: Vec<&WinRow> = ws.rows.iter().filter(|x| x.kind == WinKind::Split && x.honest).collect();
        if file.is_empty() {
            return Err(WfErr::MethodItems);
        }
        let its = file
            .iter()
            .map(|h| headline_of(views, WinId(h.idx)).map(|v| format!("{}: NOT PASSED (not run)", v.id)).ok_or(WfErr::MethodItems))
            .collect::<Result<Vec<_>, _>>()?
            .join("; ");
        let n0 = file.len();
        let head = format!("{} honest tests NOT PASSED", if n0 == 2 { "Both".to_owned() } else { format!("All {n0}") });
        return Ok(Some(vec![words.fill("W12", &[("head", &head), ("search", &search_words(part, r)), ("items", &its), ("same_combo", "no")])?]));
    }
    let n = honest.len();
    let all = !items.is_empty() && items.iter().all(|x| x.word == ItemWord::Passed);
    let head = format!("{} honest tests {}", if n == 2 { "Both".to_owned() } else { format!("All {n}") }, if all { "PASSED" } else { "NOT PASSED" });
    let its = items
        .iter()
        .map(|x| {
            let code = x.code.map_or_else(String::new, |c| c.0[0].to_string());
            match x.word {
                ItemWord::NoPick => format!("{}: NOT PASSED (no pick)", x.view),
                ItemWord::Chosen(run) => format!("{} {} {code}: NOT PASSED (not run: the pick rule was chosen after run {run} showed its test years; HINDSIGHT)", x.view, x.label),
                ItemWord::Passed => format!("{} {} {code}: PASSED", x.view, x.label),
                ItemWord::NotPassed => format!("{} {} {code}: NOT PASSED", x.view, x.label),
            }
        })
        .collect::<Vec<_>>()
        .join("; ");
    let same = items.first().and_then(|a| a.code).is_some_and(|c| items.iter().all(|x| x.code == Some(c)));
    let mut out = vec![words.fill("W12", &[("head", &head), ("search", &search_words(part, r)), ("items", &its), ("same_combo", if same { "yes" } else { "no" })])?];
    if n >= 2 {
        let mut later = honest.first().copied();
        for h in honest {
            if later.is_some_and(|l| h.learn.map_or(0, |x| x.1) > l.learn.map_or(0, |x| x.1)) {
                later = Some(h);
            }
        }
        if let Some(l) = later {
            let (lf, lt) = l.learn.unwrap_or((0, -1));
            let mut ov: Option<(Mk, Mk)> = None;
            for o in honest.iter().filter(|o| o.idx != l.idx) {
                let (Some(tf), Some(tt)) = (o.test_from, o.test_to) else { continue };
                let (a, b) = (lf.max(tf), lt.min(tt));
                if a <= b {
                    ov = Some(ov.map_or((a, b), |(x, y)| (x.min(a), y.max(b))));
                }
            }
            let fa: String = honest.iter().filter(|h| h.fixed_after != "-").map(|h| format!("; {}", h.fixed_after)).collect();
            let later_test = match (l.test_from, l.test_to) {
                (Some(a), Some(b)) => years_str(a, b),
                _ => "-".to_owned(),
            };
            out.push(words.fill("W12b", &[("later_test", &later_test), ("overlap", &ov.map_or_else(|| "no month".to_owned(), |(a, b)| overlap_str(a, b))), ("T", &l.test_name), ("fixed_after", &fa)])?);
        }
    }
    Ok(Some(out))
}

/// W14, the design-level hindsight line (always).
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn design_line(words: &Words, r: &RunRules) -> Result<String, WfErr> {
    words.fill("W14", &[("charts", if r.charts { ", including levels the operator named from his charts" } else { "" })])
}

/// The verdict code of W3 / the summary `{key}_verdict_code`.
#[must_use]
pub fn verdict_code(v: &Verdict) -> String {
    match v {
        Verdict::Passed { .. } => "Passed".to_owned(),
        Verdict::NotPassedYears { .. } => "NotPassedYears".to_owned(),
        Verdict::NotPassedHole { .. } => "NotPassedHole".to_owned(),
        Verdict::Figures => "Figures".to_owned(),
        Verdict::NotRun(n) => format!(
            "NotRun:{}",
            match n {
                NotRun::Hindsight => "Hindsight",
                NotRun::OtherProtocol { .. } => "OtherProtocol",
                NotRun::NoPick => "NoPick",
                NotRun::NoLearnMonth => "NoLearnMonth",
                NotRun::NoTestMonth => "NoTestMonth",
                NotRun::WayA => "WayA",
                NotRun::SetTooBig => "SetTooBig",
                NotRun::Fixed { .. } => "Fixed",
                NotRun::ChosenRule { .. } => "ChosenRule",
            }
        ),
    }
}
/// The pinned summary value (R2, X3): the line of the window its key names, without the test name.
#[must_use]
pub fn summary_value(line: &str, test_name: &str) -> String {
    line.strip_prefix(test_name).and_then(|x| x.strip_prefix(' ')).unwrap_or(line).to_owned()
}
/// W25: a summary across partitions or views (never REQ 73d, which is met only per search).
///
/// # Errors
/// `BadRow(T, 0, 9)`.
pub fn summary_line(words: &Words, k: u64, n: u64, what: &str, test_name: &str) -> Result<String, WfErr> {
    words.fill("W25", &[("N", &n.to_string()), ("what", what), ("T", test_name), ("k", &k.to_string())])
}

/// What the luck line reads besides the pick's trades (P23).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LuckCtx {
    /// Eligible offers of the pick's view.
    pub eligible: u64,
    /// Offers.
    pub offered: u64,
    /// Views of the run.
    pub views: u64,
    /// Partitions of the run.
    pub partitions: u64,
    /// The twin's seeds and in how many its pick had every test year positive (None: not built).
    pub twin: Option<(u64, u64)>,
    /// The mirror's pick: every test year positive (None: not built; Some(None): no trade).
    pub mirror: Option<Option<bool>>,
}
fn top_shares(nets: &mut [i64], total: i64) -> Result<[String; 3], WfErr> {
    nets.sort_unstable_by(|a, b| b.cmp(a));
    let mut out = [String::new(), String::new(), String::new()];
    for (i, k) in [1usize, 3, 5].iter().enumerate() {
        let s = sum(nets.get(..(*k).min(nets.len())).unwrap_or(&[]))?;
        if let Some(x) = out.get_mut(i) {
            *x = share_bp(s, total).map_or_else(|| "n/a".to_owned(), fmt_pct);
        }
    }
    Ok(out)
}
/// W18, the luck line of a split pick (P23): no verdict phrase in it.
///
/// # Errors
/// `Overflow`, `BadRow(T, 0, 9)`.
pub fn luck_line(words: &Words, w: &Windows, v: WinId, trades: &[Trade], f: &Facts, c: &LuckCtx) -> Result<String, WfErr> {
    let s = w.spec(v).ok_or(WfErr::SetMismatch)?;
    let (tf, tt) = s.test.ok_or(WfErr::SetMismatch)?;
    let (lf, lt) = s.learn.ok_or(WfErr::SetMismatch)?;
    let mut test: Vec<i64> = trades.iter().filter(|t| (tf..=tt).contains(&mk_of_day(t.exit_day))).map(|t| t.net).collect();
    let mut learn: Vec<i64> = trades.iter().filter(|t| (lf..=lt).contains(&mk_of_day(t.exit_day))).map(|t| t.net).collect();
    let tn = sum(&test)?;
    let ln = sum(&learn)?;
    let [t1, t3, t5] = top_shares(&mut test, tn)?;
    let [l1, l3, l5] = top_shares(&mut learn, ln)?;
    let mut best = Vec::new();
    for u in &s.test_list {
        let cell = f.units.get(usize::from(*u)).copied().unwrap_or(crate::acc::UnitCell::EMPTY);
        if cell.trades > 0 && cell.net > 0 && cell.best != i64::MIN && sub(cell.net, cell.best)? <= 0 {
            let label = w.tab.units.get(usize::from(*u)).map_or("", |x| x.label.as_str());
            best.push(format!("{label} ({} without its best trade {})", fmt_rs(sub(cell.net, cell.best)?), fmt_rs(cell.best)));
        }
    }
    let twin = c.twin.map_or_else(|| "not built in this run (named)".to_owned(), |(seeds, k)| format!("the twin's pick had every test year positive in {k} of {seeds} seeds"));
    let mirror = match c.mirror {
        None => "not built in this run (named)".to_owned(),
        Some(None) => "no trade (named)".to_owned(),
        Some(Some(b)) => format!("its pick's test years every one positive: {}", if b { "yes" } else { "no" }),
    };
    let seeds = c.twin.map_or_else(|| "0".to_owned(), |(s, _)| s.to_string());
    words.fill(
        "W18",
        &[
            ("top1", &t1),
            ("top3", &t3),
            ("top5", &t5),
            ("test_net", &fmt_rs(tn)),
            ("best_only", &if best.is_empty() { "none".to_owned() } else { best.join(", ") }),
            ("eligible", &c.eligible.to_string()),
            ("offered", &c.offered.to_string()),
            ("views", &c.views.to_string()),
            ("partitions", &c.partitions.to_string()),
            ("seeds", &seeds),
            ("twin", &twin),
            ("mirror", &mirror),
            ("ltop1", &l1),
            ("ltop3", &l3),
            ("ltop5", &l5),
            ("learn_net", &fmt_rs(ln)),
        ],
    )
}

/// Is grid `g` a SEARCH grid?
#[must_use]
pub fn is_search(r: &RunRules, g: u8) -> bool {
    r.grids.get(usize::from(g)).is_some_and(|x| x.role == GridRole::Search)
}
