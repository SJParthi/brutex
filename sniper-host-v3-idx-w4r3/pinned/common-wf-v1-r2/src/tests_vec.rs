//! Gate G0-WF, the vectors: `vectors/COMMON-WF-VECTORS-v1.tsv` (written by the INDEPENDENT node reference `vectors/wf-ref.mjs`)
//! reproduced byte for byte by the crate. This harness is the Rust twin of `runCase` in `wf-ref.mjs`: it builds the calendar, the
//! window sets, the views, the words, RULES and the partitions from each row, walks every subject through `Acc`, offers it through
//! `Lists`, and prints the same canonical lines from the crate's API (verdict, render, method line, ...). Under `--features red` it
//! compares with the row's `expect_red` (the node reference's red reading), counts the rows whose red output differs from the
//! standard, prints the count and FAILS (the red round).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::type_complexity, clippy::cast_lossless, clippy::print_stdout, reason = "tests")]

use std::collections::BTreeMap;

use crate::date::{mk_of_day, mk_str, parse_day, parse_mk};
use crate::report::{self, MethodItem};
use crate::rules::{PartitionDef, RunRules, parse_partitions, check_paper_from};
use crate::views::{Lists, Role, Tickets, Views};
use crate::windows::{NotRunWhy, WinKind, WinRow, WindowSets, Windows, parse_windows};
use crate::words::Words;
use crate::*;

pub(crate) const ROOT: &str = env!("CARGO_MANIFEST_DIR");

pub(crate) fn read(rel: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{rel}")).unwrap_or_else(|e| panic!("{rel}: {e}"))
}
pub(crate) fn windows_text(n: &str) -> String {
    if n == "STD" { read("data/WINDOWS-STD-v1.tsv") } else { read(&format!("vectors/data/WINDOWS-{n}.tsv")) }
}
pub(crate) fn views_text(n: &str) -> String {
    if n == "STD" { read("data/VIEWS-STD-v1.tsv") } else { read(&format!("vectors/data/VIEWS-{n}.tsv")) }
}

/// The calendar spec of the vectors (wf-ref.mjs `calendarFromSpec`).
pub(crate) fn calendar_rows(spec: &str) -> Vec<CalRow> {
    let mut days: BTreeMap<Day, CalRow> = BTreeMap::new();
    for part in spec.split(';') {
        let c: Vec<&str> = part.split(':').collect();
        let one = |k: DayKind, w: bool, r: bool| CalRow { day: parse_day(c[1]).unwrap(), kind: k, walked: w, ruling: r };
        match c[0] {
            "wd" => {
                let (a, b) = (parse_day(c[1]).unwrap(), parse_day(c[2]).unwrap());
                for z in a..=b {
                    let w = ((z % 7) + 7 + 4) % 7;
                    if (1..=5).contains(&w) {
                        days.insert(z, CalRow { day: z, kind: DayKind::Regular, walked: true, ruling: false });
                    }
                }
            }
            "x" => {
                let (a, b) = (parse_day(c[1]).unwrap(), parse_day(c[2]).unwrap());
                for z in a..=b {
                    days.remove(&z);
                }
            }
            "s" => { let r = one(DayKind::Special, false, false); days.insert(r.day, r); }
            "h" => { let r = one(DayKind::Halted, true, false); days.insert(r.day, r); }
            "sw" => { let r = one(DayKind::Special, true, false); days.insert(r.day, r); }
            "swr" => { let r = one(DayKind::Special, true, true); days.insert(r.day, r); }
            o => panic!("calendar spec {o}"),
        }
    }
    days.into_values().collect()
}

pub(crate) fn rules_kv(s: &str) -> Vec<(String, String)> {
    s.split('&').filter(|x| !x.is_empty()).map(|x| { let i = x.find('=').unwrap(); (x[..i].to_owned(), x[i + 1..].to_owned()) }).collect()
}

pub(crate) fn partitions_of(spec: &str) -> Result<Vec<PartitionDef>, WfErr> {
    let cols = crate::rules::PART_COLS;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for one in spec.split('|').filter(|x| !x.is_empty()) {
        let mut m: BTreeMap<&str, String> = cols.iter().map(|c| (*c, "-".to_owned())).collect();
        for (k, v) in [("id", "1"), ("role", "REAL"), ("world", "real"), ("reading", "booked"), ("side_dial", "no"), ("side_setting", "both")] {
            m.insert(k, v.to_owned());
        }
        for kv in one.split(',') {
            let i = kv.find('=').unwrap();
            let k = &kv[..i];
            let key = cols.iter().find(|c| **c == k).unwrap_or_else(|| panic!("partition column {k}"));
            m.insert(key, kv[i + 1..].to_owned());
        }
        rows.push(cols.iter().map(|c| m[c].clone()).collect());
    }
    parse_partitions(&rows)
}

#[derive(Clone, Debug)]
pub(crate) enum Ev {
    T(Trade),
    M(Day, Paisa),
}
#[derive(Clone, Debug)]
pub(crate) struct Subj {
    pub(crate) code: u64,
    pub(crate) tickets: Vec<(Option<u8>, u8, ReadSet)>,
    pub(crate) span: Option<(Day, Day)>,
    pub(crate) part: u32,
    pub(crate) fixed: Option<String>,
    pub(crate) host: Vec<(String, i64)>,
    pub(crate) ev: Vec<Ev>,
}
pub(crate) fn read_set(s: &str) -> ReadSet {
    if s == "-" { ReadSet::EMPTY } else { let (a, b) = s.split_once("..").unwrap(); ReadSet { from: parse_mk(a).unwrap(), to: parse_mk(b).unwrap() } }
}
pub(crate) fn subjects(s: &str) -> Vec<Subj> {
    s.split('|').filter(|x| !x.is_empty()).map(|one| {
        let mut o = Subj { code: 0, tickets: Vec::new(), span: None, part: 1, fixed: None, host: Vec::new(), ev: Vec::new() };
        for f in one.split('&') {
            let i = f.find('=').unwrap();
            let (k, v) = (&f[..i], &f[i + 1..]);
            match k {
                "code" => o.code = v.parse().unwrap(),
                "tickets" => o.tickets = v.split(',').filter(|x| !x.is_empty()).map(|t| {
                    let c: Vec<&str> = t.split(':').collect();
                    let g = if c[0] == "NO" { None } else { Some(c[0].parse::<u8>().unwrap()) };
                    let st = if c[1] == "-" { NO_STAGE } else { c[1].parse().unwrap() };
                    (g, st, read_set(c[2]))
                }).collect(),
                "span" => { let (a, b) = v.split_once("..").unwrap(); o.span = Some((parse_day(a).unwrap(), parse_day(b).unwrap())); }
                "part" => o.part = v.parse().unwrap(),
                "fixed" => o.fixed = Some(v.to_owned()),
                "host" => o.host = v.split(',').map(|h| { let (a, b) = h.split_once(':').unwrap(); (a.to_owned(), b.parse().unwrap()) }).collect(),
                "ev" => o.ev = v.split(',').filter(|x| !x.is_empty()).map(|e| {
                    let c: Vec<&str> = e.split(':').collect();
                    match c[0] {
                        "T" => Ev::T(Trade { day: parse_day(c[1]).unwrap(), exit_day: parse_day(c[2]).unwrap(), close_s: c[3].parse().unwrap(), net: c[4].parse().unwrap(), net_alt: 0, rd_bp: c[5].parse().unwrap(), hole: c[6] == "1", id: c[7].parse().unwrap(), mark_prev: c.get(8).map_or(0, |x| x.parse().unwrap()) }),
                        "M" => Ev::M(parse_day(c[1]).unwrap(), c[2].parse().unwrap()),
                        o => panic!("event {o}"),
                    }
                }).collect(),
                o => panic!("subject field {o}"),
            }
        }
        o
    }).collect()
}
pub(crate) fn code(c: u64) -> Code { Code([c, 0, 0, 0]) }

/// Walks one subject into a fresh `Acc` of window set `w` (the search walk and the report re-walk are the same call sequence, 5.1).
pub(crate) fn walk(w: &Windows, v: &Views, r: &RunRules, s: &Subj) -> Result<Acc, WfErr> {
    let mut acc = Acc::new(w, v, r);
    acc.reset();
    if let Some((a, b)) = s.span {
        acc.set_data_span(a, b)?;
    }
    for e in &s.ev {
        match e {
            Ev::T(t) => { acc.add(t)?; }
            Ev::M(d, m) => { acc.add_session_mtm(*d, *m)?; }
        }
    }
    acc.finish()?;
    Ok(acc)
}

pub(crate) struct Case<'a> {
    pub(crate) windows: &'a str,
    pub(crate) views: &'a str,
    pub(crate) cal: &'a str,
    pub(crate) rules: &'a str,
    pub(crate) partitions: &'a str,
    pub(crate) subjects: &'a str,
    pub(crate) report: &'a str,
    pub(crate) ops: &'a str,
    pub(crate) host: &'a str,
}

struct Ctx {
    rules: RunRules,
    words: Words,
    ws: WindowSets,
    views: Views,
    parts: Vec<PartitionDef>,
}

/// The canonical driver (wf-ref.mjs `runCase`).
pub(crate) fn run_case(c: &Case<'_>, windows: &dyn Fn(&str) -> String, views: &dyn Fn(&str) -> String) -> Vec<String> {
    let mut out = Vec::new();
    if let Err(e) = run_inner(c, windows, views, &mut out) {
        out.push(format!("E load: {e}"));
    }
    out
}

fn run_inner(c: &Case<'_>, windows_f: &dyn Fn(&str) -> String, views_f: &dyn Fn(&str) -> String, out: &mut Vec<String>) -> Result<(), WfErr> {
    let mut rules = RunRules::from_rows(&rules_kv(c.rules))?;
    let cal = Calendar::from_rows(&calendar_rows(c.cal))?;
    let rows: Vec<WinRow> = parse_windows(&windows_f(c.windows))?;
    let host: Vec<&str> = c.host.split(',').filter(|x| !x.is_empty()).collect();
    let views = Views::from_text(&views_f(c.views), &rows, &rules, &host)?;
    let words = Words::from_text(&read("data/WORDS-STD-v1.tsv"))?;
    let parts = partitions_of(c.partitions)?;
    rules.partitions.clone_from(&parts);
    check_paper_from(&rules, &rows, &views)?;
    let ws = Windows::sets_from_rows(&rows, &cal, rules.period)?;
    let cx = Ctx { rules, words, ws, views, parts };
    let subs = subjects(c.subjects);
    let want = |v: &crate::views::View| -> bool {
        if c.report == "*" { true } else if !c.report.is_empty() { c.report.split(',').any(|x| x == v.id) } else { matches!(v.role, Role::Headline | Role::Miss) }
    };
    let mut lists: Vec<Lists> = cx.ws.sets.iter().map(|w| Lists::new(&cx.views, w, &cx.rules, 0)).collect();
    let mut walked: BTreeMap<u64, Subj> = BTreeMap::new();
    let mut fixed_subs: Vec<Subj> = Vec::new();
    for s in &subs {
        // the walk of every set (feed errors are the subject's)
        let mut accs = Vec::new();
        let mut bad = None;
        for w in &cx.ws.sets {
            match walk(w, &cx.views, &cx.rules, s) {
                Ok(a) => accs.push(a),
                Err(e) => { bad = Some(e); break; }
            }
        }
        if let Some(e) = bad {
            out.push(format!("E subj {}: {e}", s.code));
            continue;
        }
        walked.insert(s.code, s.clone());
        let mut tk = Tickets::new();
        let mut terr = None;
        for (g, st, set) in &s.tickets {
            let t = Ticket { grid: g.unwrap_or(NO_GRID), stage: *st, set: *set };
            if let Err(e) = tk.push(t, &cx.rules) {
                terr = Some(e);
                break;
            }
        }
        if s.fixed.is_some() {
            fixed_subs.push(s.clone());
            continue;
        }
        match terr {
            Some(WfErr::TooManyTickets) => { out.push(format!("E tickets {}: TooManyTickets", s.code)); continue; }
            Some(WfErr::FixedOffered) => out.push(format!("E tickets {}: FixedOffered", s.code)),
            Some(e) => { out.push(format!("E tickets {}: {e}", s.code)); continue; }
            None => {}
        }
        let host_vals: Vec<i64> = cx.views.host_names.iter().map(|n| s.host.iter().find(|(a, _)| a == n).map_or(0, |(_, b)| *b)).collect();
        for (si, acc) in accs.iter().enumerate() {
            match lists[si].offer_host(&cx.views, Partition(s.part), &tk, code(s.code), acc, &host_vals) {
                Ok(_) | Err(WfErr::FixedOffered) => {}
                Err(e) => { out.push(format!("E offer {}: {e}", s.code)); break; }
            }
        }
    }
    let honest: Vec<WinId> = cx.ws.honest();
    let set_of = |w: WinId| -> Option<usize> { cx.ws.sets.iter().position(|s| s.has(w)) };
    for p in &cx.parts {
        let mut m_items: BTreeMap<u8, MethodItem> = BTreeMap::new();
        for (vi, v) in cx.views.views.iter().enumerate() {
            let vid = ViewId(u16::try_from(vi).unwrap());
            let Some(si) = set_of(v.win) else {
                if want(v) {
                    let why = cx.ws.not_run.iter().find(|(w, _)| *w == v.win).map_or(NotRunWhy::Test, |x| x.1);
                    out.push(format!("R {} W08: {}", v.id, report::not_run_line(&cx.words, cx.ws.row(v.win), why)?));
                }
                continue;
            };
            let w = &cx.ws.sets[si];
            let l = &lists[si];
            let cn = l.counts(Partition(p.id), vid);
            let kept = l.kept(Partition(p.id), vid);
            if want(v) {
                out.push(format!("L {} p={}: kept={} ties={} offered={} eligible={} rs={} ng={} cg={} fixed={} nolearn={} year={} floor={}", v.id, p.id, kept.len(), l.ties(Partition(p.id), vid), cn.offered, cn.eligible, cn.rejected_read_set, cn.rejected_no_grid, cn.rejected_chosen_grid, cn.fixed_offered, cn.rejected_no_learn_trade, cn.rejected_learn_year, cn.rejected_floor));
            }
            if v.role == Role::Miss {
                if want(v) {
                    for k in kept {
                        out.push(format!("X {} {}: {}", v.id, k.key.code.0[0], report::miss_checks(w, v.win, &k.facts).join("; ")));
                    }
                }
                continue;
            }
            let Some(top) = kept.first() else {
                if want(v) {
                    out.push(format!("R {} W07: {}", v.id, report::no_pick_line(&cx.words, v, cx.ws.row(v.win), &cn, p, &cx.rules)?));
                }
                if v.role == Role::Headline && cx.ws.row(v.win).honest {
                    m_items.insert(v.win.0, MethodItem { win: v.win, partition: Partition(p.id), view: v.id.clone(), label: cx.ws.row(v.win).pick_label.clone(), code: None, word: report::ItemWord::NoPick });
                }
                continue;
            };
            let learn = w.learn_range(v.win).unwrap_or((0, -1));
            let pick = PickWin { set: w.set_id(), win: v.win, learn, kind: top.kind, code: top.key.code };
            let subj = &walked[&top.key.code.0[0]];
            if want(v) {
                out.push(format!("P {} p={} #1={} key={} fp={:x}", v.id, p.id, top.key.code.0[0], top.key.parts[..usize::from(top.key.n)].iter().map(i64::to_string).collect::<Vec<_>>().join(","), top.learn.fp));
                let re = walk(w, &cx.views, &cx.rules, subj)?;
                assert!(re.facts(Partition(p.id)) == top.facts, "rewalk_equal");
                for (ri, r) in cx.ws.rows.iter().enumerate() {
                    let wid = WinId(u8::try_from(ri).unwrap());
                    let Some(rsi) = set_of(wid) else { continue };
                    let rw = &cx.ws.sets[rsi];
                    let racc = if rsi == si { None } else { Some(walk(rw, &cx.views, &cx.rules, subj)?) };
                    let acc = racc.as_ref().unwrap_or(&re);
                    let facts = acc.facts(Partition(p.id));
                    let lines = match verdict(rw, wid, pick, &facts, &cx.rules) {
                        Ok(rv) => report::reading_lines(&cx.words, rw, wid, &rv, &pick, cx.ws.row(v.win), &facts, acc, &cx.rules, p, "", None),
                        Err(e) => Err(e),
                    };
                    match lines {
                        Ok(ls) => for ln in ls { out.push(format!("R {} {}: {ln}", v.id, r.id)); },
                        Err(e) => out.push(format!("R {} {}: error {e}", v.id, r.id)),
                    }
                }
                let n_open = report::open_at_learn_end(&trades_of(subj), learn.1);
                out.push(format!("R {} label: {}", v.id, report::pick_label(&cx.words, cx.ws.row(v.win), learn, top, n_open, &cx.rules)?));
                out.push(format!("R {} ties: {}", v.id, report::ties_line(&cx.words, l.ties(Partition(p.id), vid))?));
                out.push(format!("R {} margin: {}", v.id, report::margin_line(&cx.words, kept, v)?));
                let row = cx.ws.row(v.win);
                if row.kind == WinKind::Split {
                    out.push(format!("R {} holdout: {}", v.id, report::holdout_line(&cx.words, row, &cx.rules)?));
                    if row.fixed_after != "-" {
                        out.push(format!("R {} fixed_after: {}", v.id, row.fixed_after));
                    }
                    for line in report::knowingly_lines(&cx.words, p, &honest.iter().map(|h| cx.ws.row(*h)).collect::<Vec<_>>())? {
                        out.push(format!("R {} knowingly: {line}", v.id));
                    }
                }
            }
            if v.role == Role::Headline && cx.ws.row(v.win).honest {
                let (_, vd) = verdict(w, v.win, pick, &top.facts, &cx.rules)?;
                let word = match vd {
                    Verdict::Passed { .. } => report::ItemWord::Passed,
                    Verdict::NotRun(NotRun::ChosenRule { run }) => report::ItemWord::Chosen(run),
                    _ => report::ItemWord::NotPassed,
                };
                m_items.insert(v.win.0, MethodItem { win: v.win, partition: Partition(p.id), view: v.id.clone(), label: cx.ws.row(v.win).pick_label.clone(), code: Some(top.key.code), word });
            }
        }
        let items: Vec<MethodItem> = honest.iter().filter_map(|h| m_items.get(&h.0).cloned()).collect();
        if let Some(ml) = report::method_line(&cx.words, &items, &cx.ws, &cx.views, p, &cx.rules)? {
            for m in ml {
                out.push(format!("M p={}: {m}", p.id));
            }
        }
    }
    // FIXED subjects (P13, W26): walked for the report only, never offered
    for fs in &fixed_subs {
        let gi = cx.rules.fixed.iter().find(|f| f.code == code(fs.code)).map_or_else(|| fs.tickets.first().and_then(|t| t.0).unwrap_or(0), |f| f.grid);
        let twin = (0..cx.rules.grids.len()).find(|&i| i != usize::from(gi) && cx.rules.grids[i].role == GridRole::Search && fs.tickets.iter().any(|t| t.0 == Some(u8::try_from(i).unwrap())));
        let note = twin.map_or_else(String::new, |i| report::fixed_overlap_note(&cx.rules, u8::try_from(i).unwrap()));
        let part = cx.parts.iter().find(|p| p.id == fs.part).unwrap();
        for (ri, r) in cx.ws.rows.iter().enumerate() {
            let wid = WinId(u8::try_from(ri).unwrap());
            if r.kind != WinKind::Split { continue; }
            let Some(rsi) = set_of(wid) else { continue };
            let rw = &cx.ws.sets[rsi];
            let acc = walk(rw, &cx.views, &cx.rules, fs)?;
            let facts = acc.facts(Partition(fs.part));
            let pick = PickWin { set: rw.set_id(), win: WinId(u8::MAX), learn: (0, -1), kind: PickKind::Fixed { grid: gi }, code: code(fs.code) };
            let rv = verdict(rw, wid, pick, &facts, &cx.rules)?;
            for ln in report::reading_lines(&cx.words, rw, wid, &rv, &pick, r, &facts, &acc, &cx.rules, part, &note, None)? {
                out.push(format!("F {} {}: {ln}", fs.fixed.as_deref().unwrap_or("-"), r.id));
            }
        }
    }
    out.push(format!("D {}", report::design_line(&cx.words, &cx.rules)?));
    for op in c.ops.split(';').filter(|x| !x.is_empty()) {
        run_op(op, &cx, &lists, &walked, out)?;
    }
    Ok(())
}

pub(crate) fn trades_of(s: &Subj) -> Vec<Trade> {
    s.ev.iter().filter_map(|e| if let Ev::T(t) = e { Some(*t) } else { None }).collect()
}

fn run_op(op: &str, cx: &Ctx, lists: &[Lists], walked: &BTreeMap<u64, Subj>, out: &mut Vec<String>) -> Result<(), WfErr> {
    let (name, arg) = op.split_once('=').unwrap();
    let p = &cx.parts[0];
    let find = |vid: &str| -> (usize, ViewId, &crate::views::View) {
        let vi = cx.views.views.iter().position(|x| x.id == vid).unwrap();
        let v = &cx.views.views[vi];
        (cx.ws.sets.iter().position(|s| s.has(v.win)).unwrap(), ViewId(u16::try_from(vi).unwrap()), v)
    };
    match name {
        "summary" => {
            let (vid, key) = arg.split_once(':').unwrap();
            let (si, view, v) = find(vid);
            let w = &cx.ws.sets[si];
            let Some(top) = lists[si].kept(Partition(p.id), view).first() else { out.push(format!("S {key}: no pick")); return Ok(()) };
            let pick = PickWin { set: w.set_id(), win: v.win, learn: w.learn_range(v.win).unwrap_or((0, -1)), kind: top.kind, code: top.key.code };
            let acc = walk(w, &cx.views, &cx.rules, &walked[&top.key.code.0[0]])?;
            for h in cx.ws.honest() {
                let rv = verdict(w, h, pick, &top.facts, &cx.rules)?;
                let r = cx.ws.row(h);
                let lines = report::reading_lines(&cx.words, w, h, &rv, &pick, cx.ws.row(v.win), &top.facts, &acc, &cx.rules, p, "", None)?;
                let code = report::verdict_code(&rv.1);
                if r.id == "WF" {
                    out.push(format!("S {key}_verdict {}", report::summary_value(&lines[0], &r.test_name)));
                    out.push(format!("S {key}_verdict_code {code}"));
                } else if matches!(rv.1, Verdict::Figures) {
                    out.push(format!("S {key}_tt_figures_all_pos {}", if report::all_pos(w, h, &top.facts, &cx.rules) { "yes" } else { "no" }));
                } else {
                    out.push(format!("S {key}_tt_verdict {}", report::summary_value(&lines[0], &r.test_name)));
                    out.push(format!("S {key}_tt_verdict_code {code}"));
                }
            }
        }
        "replays" => {
            let (vid, list) = arg.split_once(':').unwrap();
            let (si, view, v) = find(vid);
            let w = &cx.ws.sets[si];
            let top = &lists[si].kept(Partition(p.id), view)[0];
            let pick = PickWin { set: w.set_id(), win: v.win, learn: w.learn_range(v.win).unwrap_or((0, -1)), kind: top.kind, code: top.key.code };
            let reps: Vec<(u8, Facts)> = list.split(',').enumerate().map(|(i, c)| (u8::try_from(i).unwrap(), walk(w, &cx.views, &cx.rules, &walked[&c.parse::<u64>().unwrap()]).unwrap().facts(Partition(p.id)))).collect();
            let rv = verdict_with_replays(w, v.win, pick, &top.facts, &reps, &cx.rules)?;
            let level = if let Verdict::NotPassedYears { at_level: Some(l), .. } = rv.1 { reps.get(usize::from(l)).map(|x| &x.1) } else { None };
            let acc = walk(w, &cx.views, &cx.rules, &walked[&top.key.code.0[0]])?;
            let lines = report::reading_lines(&cx.words, w, v.win, &rv, &pick, cx.ws.row(v.win), &top.facts, &acc, &cx.rules, p, "", level)?;
            out.push(format!("Q {vid}: {}", lines[0]));
        }
        "tickets" => {
            let mut tk = Tickets::new();
            let mut err = "-".to_owned();
            for t in arg.split('/') {
                let c: Vec<&str> = t.split(':').collect();
                let g = if c[0] == "NO" { NO_GRID } else { c[0].parse().unwrap() };
                let st = if c[1] == "-" { NO_STAGE } else { c[1].parse().unwrap() };
                if let Err(e) = tk.push(Ticket { grid: g, stage: st, set: read_set(c[2]) }, &cx.rules) {
                    err = e.to_string();
                    break;
                }
            }
            let kept = tk.kept().iter().map(|t| format!("{}:{}", if t.grid == NO_GRID { "NO".to_owned() } else { format!("g{}", t.grid) }, if t.set.is_empty() { "-".to_owned() } else { format!("{}..{}", mk_str(t.set.from), mk_str(t.set.to)) })).collect::<Vec<_>>().join(",");
            out.push(format!("K kept={kept} dominated={} no_grid={} fixed={} err={err}", tk.dominated, tk.no_grid, tk.fixed_offered));
        }
        "list" => {
            let (si, view, _) = find(arg);
            out.push(format!("O {arg}: {}", lists[si].kept(Partition(p.id), view).iter().map(|k| k.key.code.0[0].to_string()).collect::<Vec<_>>().join(",")));
        }
        "method" => {
            // method=ROW:WORD:PART:CODE[:VIEW]/... (repair of check-build-v1 B1; repair 2, check-build-v1-r2 B1-R: an optional VIEW
            // names the item's view, else the row's HEADLINE view): the method line over the items a host gives, as given; the
            // honest rows are the run's own (method_line reads them from the window sets)
            let mut items = Vec::new();
            for it in arg.split('/').filter(|x| !x.is_empty()) {
                let c: Vec<&str> = it.split(':').collect();
                let wi = cx.ws.rows.iter().position(|r| r.id == c[0]).unwrap();
                let row = &cx.ws.rows[wi];
                let view = c.get(4).map_or_else(|| cx.views.views.iter().find(|v| usize::from(v.win.0) == wi && v.role == Role::Headline).map_or_else(|| "-".to_owned(), |v| v.id.clone()), |v| (*v).to_owned());
                let word = match c[1] {
                    "P" => report::ItemWord::Passed,
                    "N" => report::ItemWord::NotPassed,
                    "X" => report::ItemWord::NoPick,
                    w => report::ItemWord::Chosen(w[1..].parse().unwrap()),
                };
                let cd = if c[3] == "-" { None } else { Some(code(c[3].parse().unwrap())) };
                let part: u32 = c[2].parse().unwrap();
                items.push(MethodItem { win: WinId(u8::try_from(wi).unwrap()), partition: Partition(part), view, label: row.pick_label.clone(), code: cd, word });
            }
            match report::method_line(&cx.words, &items, &cx.ws, &cx.views, p, &cx.rules) {
                Ok(Some(ls)) => {
                    for l in ls {
                        out.push(format!("N p={}: {l}", p.id));
                    }
                }
                Ok(None) => out.push(format!("N p={}: none", p.id)),
                Err(e) => out.push(format!("N p={}: error {e}", p.id)),
            }
        }
        "summary25" => {
            let c: Vec<&str> = arg.split(':').collect();
            out.push(format!("U {}", report::summary_line(&cx.words, c[0].parse().unwrap(), c[1].parse().unwrap(), c[2], c[3])?));
        }
        o => panic!("op {o}"),
    }
    let _ = mk_of_day(0);
    Ok(())
}

pub(crate) struct VecRow {
    pub(crate) id: String,
    pub(crate) cols: Vec<String>,
}
pub(crate) fn vector_rows() -> Vec<VecRow> {
    let text = read("vectors/COMMON-WF-VECTORS-v1.tsv");
    let mut it = text.lines();
    let head: Vec<&str> = it.next().unwrap().split('\t').collect();
    assert_eq!(head, ["id", "title", "windows", "views", "cal", "rules", "partitions", "subjects", "report", "ops", "host", "expect", "expect_red"]);
    it.filter(|l| !l.is_empty()).map(|l| { let c: Vec<String> = l.split('\t').map(str::to_owned).collect(); assert_eq!(c.len(), 13, "{}", c[0]); VecRow { id: c[0].clone(), cols: c } }).collect()
}

#[test]
fn g0_vectors_byte_for_byte() {
    let rows = vector_rows();
    let mut fails = Vec::new();
    let mut red_differs = 0usize;
    let mut node_red_differs = 0usize;
    for r in &rows {
        let c = &r.cols;
        let case = Case { windows: &c[2], views: &c[3], cal: &c[4], rules: &c[5], partitions: &c[6], subjects: &c[7], report: &c[8], ops: &c[9], host: &c[10] };
        let got = run_case(&case, &windows_text, &views_text).join("\\n");
        let want = if RED { &c[12] } else { &c[11] };
        if c[11] != c[12] { node_red_differs += 1; }
        if RED && got != c[11] { red_differs += 1; }
        if got != *want {
            let g: Vec<&str> = got.split("\\n").collect();
            let w: Vec<&str> = want.split("\\n").collect();
            let first = (0..g.len().max(w.len())).find(|&i| g.get(i) != w.get(i)).unwrap_or(0);
            fails.push(format!("{}: first difference at line {first}:\n  got  {}\n  want {}", r.id, g.get(first).unwrap_or(&"<none>"), w.get(first).unwrap_or(&"<none>")));
        }
    }
    println!("G0 vectors: {} rows, {} reproduced, {} failed", rows.len(), rows.len() - fails.len(), fails.len());
    for f in &fails {
        println!("FAIL {f}");
    }
    if RED {
        println!("RED ROUND: the red build differs from the standard on {red_differs} vectors (the node reference: {node_red_differs})");
        assert_eq!(red_differs, node_red_differs, "the red count equals the node reference's");
        panic!("the red round fails by design: {red_differs} vectors differ from the standard");
    }
    assert!(fails.is_empty(), "{} vectors failed", fails.len());
}

fn file_rows(rel: &str) -> Vec<VecRow> {
    let text = read(rel);
    text.lines().skip(1).filter(|l| !l.is_empty()).map(|l| { let c: Vec<String> = l.split('\t').map(str::to_owned).collect(); assert_eq!(c.len(), 13, "{}", c[0]); VecRow { id: c[0].clone(), cols: c } }).collect()
}

#[test]
fn g0_matrix_every_cell_equals_the_node_reference() {
    // plan 11.2-11.3: every matrix cell equals the node reference; under `red` the count of cells whose output differs from the
    // standard equals the node reference's red count, and the red round FAILS by design.
    let rows = file_rows("matrix/MATRIX-WF-v1.tsv");
    let (mut fails, mut red_differs, mut node_red) = (Vec::new(), 0usize, 0usize);
    for r in &rows {
        let c = &r.cols;
        let case = Case { windows: &c[2], views: &c[3], cal: &c[4], rules: &c[5], partitions: &c[6], subjects: &c[7], report: &c[8], ops: &c[9], host: &c[10] };
        let got = run_case(&case, &windows_text, &views_text).join("\\n");
        if c[11] != c[12] { node_red += 1; }
        if RED && got != c[11] { red_differs += 1; }
        if got != *(if RED { &c[12] } else { &c[11] }) { fails.push(r.id.clone()); }
    }
    println!("G0 matrix: {} cells, {} equal the node reference, {} differ: {:?}", rows.len(), rows.len() - fails.len(), fails.len(), fails.iter().take(20).collect::<Vec<_>>());
    if RED {
        println!("RED ROUND (matrix): the red build differs from the standard on {red_differs} cells (the node reference: {node_red})");
        assert!(fails.is_empty(), "the red build reproduces the node reference's red reading");
        assert_eq!(red_differs, node_red);
        panic!("the red round fails by design: {red_differs} matrix cells differ from the standard");
    }
    assert!(fails.is_empty(), "{} matrix cells failed", fails.len());
}
