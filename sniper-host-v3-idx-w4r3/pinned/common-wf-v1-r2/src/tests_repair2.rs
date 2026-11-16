//! The repair round of `state/common-wf-20261113/check-build-v1-r2.md` (2026-11-13, repair 2). RED FIRST: this file was written
//! against the repaired-v1 API, where `report::method_line` takes the host's honest-row slice, and run on that code before any fix
//! (`repair2-build-v1-work/RED-FIRST-run-red0.log`: B1-P1, B1-P2, B1-P3 and R1 FAILED for the named reasons; the red-first copy is
//! kept as `repair2-build-v1-work/tests_repair2.rs.red-first`, RF8). After the fix `method_line` takes the run's window sets and
//! views, so the two slice probes are now impossible by type; they are kept as the same probes over the run's own rows AND over a
//! host's hand-built `WindowSets` with a set doubled or dropped (refused). The assertions were never loosened:
//! - B1-R: the checker's probes B1-P1 (a short honest slice), B1-P2 (a doubled honest slice over a NOT PASSED train/test item) and
//!   B1-P3 (a BESIDE view's pick as the walk-forward item) never print an honest-tests line over a missing or unread item;
//! - R1: a run with no honest row in use gets a NOT PASSED method line, never a run refusal;
//! - RF1: the surviving mutants Y3 (an extra item of a non-honest row) and Y5 (a partition missing only `prov_universe`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, reason = "tests")]

use crate::report::{self, ItemWord, MethodItem};
use crate::rules::{RunRules, check_paper_from};
use crate::tests_vec::{Case, calendar_rows, partitions_of, read, run_case, rules_kv, views_text, windows_text};
use crate::views::Views;
use crate::windows::{WinKind, WinRow, WindowSets, Windows, parse_windows};
use crate::*;

const RULES_NO_DEFAULT: &str = "period=2018-09..2026-10&signed=2026-11&paper_from=2026-12&order=SEQ_LOSSES_FIRST&holes=lane&grid=1000:SEARCH:LATTICE&proof=proof step: test&data=data: complete&run=1";
const PROV_ALL: &str = "prov_account=RULED_FROM_TEXT t,prov_universe=RULED_FROM_TEXT t,prov_share_level=RULED_FROM_TEXT t,prov_exit_set=RULED_FROM_TEXT t,prov_timeframe_family=RULED_FROM_TEXT t,prov_side_setting=RULED_FROM_TEXT t,prov_fill_level=RULED_FROM_TEXT t,prov_timing=RULED_FROM_TEXT t,prov_reading=RULED_FROM_TEXT t";
const PROV_NO_UNIVERSE: &str = "prov_account=RULED_FROM_TEXT t,prov_share_level=RULED_FROM_TEXT t,prov_exit_set=RULED_FROM_TEXT t,prov_timeframe_family=RULED_FROM_TEXT t,prov_side_setting=RULED_FROM_TEXT t,prov_fill_level=RULED_FROM_TEXT t,prov_timing=RULED_FROM_TEXT t,prov_reading=RULED_FROM_TEXT t";
const CAL_STD: &str = "wd:2018-09-01:2026-10-31";

fn honest_rows(rows: &[WinRow]) -> Vec<&WinRow> {
    rows.iter().filter(|r| r.kind == WinKind::Split && r.honest).collect()
}
fn item(view: &str, row: &WinRow, part: u32, code: Option<u64>, word: ItemWord) -> MethodItem {
    MethodItem { win: WinId(row.idx), partition: Partition(part), view: view.to_owned(), label: row.pick_label.clone(), code: code.map(|c| Code([c, 0, 0, 0])), word }
}
struct Std {
    words: Words,
    rows: Vec<WinRow>,
    r: RunRules,
    views: Views,
    ws: WindowSets,
}
fn ctx(windows: &str, rules: &str) -> Std {
    let words = Words::from_text(&read("data/WORDS-STD-v1.tsv")).unwrap();
    let rows = parse_windows(&windows_text(windows)).unwrap();
    let mut r = RunRules::from_rows(&rules_kv(rules)).unwrap();
    r.partitions = partitions_of(&format!("id=1,underlying=X,account=A1,universe=X,{PROV_ALL}")).unwrap();
    let views = Views::from_text(&views_text(windows), &rows, &r, &[]).unwrap();
    let cal = Calendar::from_rows(&calendar_rows(CAL_STD)).unwrap();
    let ws = Windows::sets_from_rows(&rows, &cal, r.period).unwrap();
    Std { words, rows, r, views, ws }
}
fn std_ctx() -> Std {
    ctx("STD", RULES_NO_DEFAULT)
}

#[test]
fn b1r_p2_a_doubled_honest_slice_never_passes_over_a_not_passed_train_test_item() {
    // the checker's B1-P2: honest [WF, WF], items [WF-R73 PASSED 7, TT-R73 NOT PASSED 9]: v1-repaired printed "Both honest tests
    // PASSED" twice over the WF item and never read the TT item. Now the rows are the run's own: the TT item is read
    let s = std_ctx();
    assert_eq!(s.ws.sets.len(), 1);
    let h = honest_rows(&s.rows);
    let (wf, tt) = (h[0], h[1]);
    let p = &s.r.partitions[0];
    let items = [item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("TT-R73", tt, 1, Some(9), ItemWord::NotPassed)];
    let got = report::method_line(&s.words, &items, &s.ws, &s.views, p, &s.r).unwrap().unwrap();
    assert!(got[0].starts_with("Both honest tests NOT PASSED (") && got[0].ends_with("; WF-R73 WALK-FORWARD PICK 7: PASSED; TT-R73 TRAIN/TEST PICK 9: NOT PASSED; same combo: no)"), "B1-R: {got:?}");
    // a host's hand-built window sets naming the one set twice (every honest row doubled): refused
    let doubled = WindowSets { sets: vec![s.ws.sets[0].clone(), s.ws.sets[0].clone()], not_run: s.ws.not_run.clone(), rows: s.ws.rows.clone() };
    assert_eq!(doubled.honest().len(), 4);
    let got = report::method_line(&s.words, &items, &doubled, &s.views, p, &s.r);
    assert_eq!(got, Err(WfErr::MethodItems), "B1-R: a doubled set printed {got:?}");
    let both = [item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("TT-R73", tt, 1, Some(7), ItemWord::Passed)];
    assert_eq!(report::method_line(&s.words, &both, &doubled, &s.views, p, &s.r), Err(WfErr::MethodItems));
    // an honest row both in use and listed as not run: refused
    let mut twice = s.ws.clone();
    twice.not_run.push((WinId(tt.idx), crate::windows::NotRunWhy::Test));
    assert_eq!(report::method_line(&s.words, &both, &twice, &s.views, p, &s.r), Err(WfErr::MethodItems));
}

#[test]
fn b1r_p1_a_short_honest_slice_is_refused() {
    // the checker's B1-P1: honest [WF] (TT left out), items [WF-R73 PASSED 7]: v1-repaired printed "All 1 honest tests PASSED".
    // Over the run's own rows TT has no item: refused
    let s = std_ctx();
    let h = honest_rows(&s.rows);
    let p = &s.r.partitions[0];
    let wf_only = [item("WF-R73", h[0], 1, Some(7), ItemWord::Passed)];
    let got = report::method_line(&s.words, &wf_only, &s.ws, &s.views, p, &s.r);
    assert_eq!(got, Err(WfErr::MethodItems), "B1-R: a short item list printed {got:?}");
    // a host's hand-built window sets with the set dropped (WF and TT neither in use nor not run): refused, never read as "no
    // honest row in use"
    let dropped = WindowSets { sets: Vec::new(), not_run: s.ws.not_run.clone(), rows: s.ws.rows.clone() };
    assert_eq!(report::method_line(&s.words, &[], &dropped, &s.views, p, &s.r), Err(WfErr::MethodItems));
    assert_eq!(report::method_line(&s.words, &wf_only, &dropped, &s.views, p, &s.r), Err(WfErr::MethodItems));
    // SETS2: the honest late row RH sits in the second window set; every set's honest rows count (plan P19, Y5)
    let s2 = ctx("SETS2", RULES_NO_DEFAULT);
    let h2 = honest_rows(&s2.rows);
    assert_eq!(h2.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["WF", "TT", "RH"]);
    let rh = WinId(h2[2].idx);
    let rh_set = s2.ws.sets.iter().position(|x| x.has(rh)).unwrap();
    assert!(rh_set > 0 && !s2.ws.sets[0].has(rh), "RH must sit in a later set");
    let three = [item("WF-R73", h2[0], 1, Some(7), ItemWord::Passed), item("TT-R73", h2[1], 1, Some(7), ItemWord::Passed), item("RH-R73", h2[2], 1, Some(7), ItemWord::Passed)];
    let ok = report::method_line(&s2.words, &three, &s2.ws, &s2.views, &s2.r.partitions[0], &s2.r).unwrap().unwrap();
    assert!(ok[0].starts_with("All 3 honest tests PASSED (") && ok[0].ends_with("; RH-R73 LATE PICK 7: PASSED; same combo: yes)"), "{ok:?}");
    // RH's item left out: refused; RH's set dropped by a host: refused (never "Both honest tests PASSED" over WF and TT)
    assert_eq!(report::method_line(&s2.words, &three[..2], &s2.ws, &s2.views, &s2.r.partitions[0], &s2.r), Err(WfErr::MethodItems));
    let mut short = s2.ws.clone();
    short.sets.remove(rh_set);
    assert_eq!(report::method_line(&s2.words, &three[..2], &short, &s2.views, &s2.r.partitions[0], &s2.r), Err(WfErr::MethodItems));
    assert_eq!(report::method_line(&s2.words, &three, &short, &s2.views, &s2.r.partitions[0], &s2.r), Err(WfErr::MethodItems));
}

#[test]
fn b1r_p3_only_the_rows_headline_pick_counts() {
    // the checker's B1-P3: a BESIDE view's pick (WF-R73-F8) on the WF row; also a MISS view, a view of another row, an unknown view
    let s = std_ctx();
    let h = honest_rows(&s.rows);
    let (wf, tt) = (h[0], h[1]);
    let p = &s.r.partitions[0];
    for v in ["WF-R73-F8", "WF-MISS", "TT-R73", "WF-R74", "-"] {
        let got = report::method_line(&s.words, &[item(v, wf, 1, Some(7), ItemWord::Passed), item("TT-R73", tt, 1, Some(7), ItemWord::Passed)], &s.ws, &s.views, p, &s.r);
        assert_eq!(got, Err(WfErr::MethodItems), "B1-R: a {v} item on the walk-forward row printed {got:?}");
    }
}

#[test]
fn rf1_y3_an_extra_item_of_a_non_honest_row_is_refused() {
    // the checker's surviving mutant Y3: one item per honest row PLUS an item of the HS row is refused, never silently dropped
    let s = std_ctx();
    let h = honest_rows(&s.rows);
    let hs = s.rows.iter().find(|x| x.id == "HS").unwrap();
    let p = &s.r.partitions[0];
    let items = [item("WF-R73", h[0], 1, Some(7), ItemWord::Passed), item("TT-R73", h[1], 1, Some(7), ItemWord::Passed), item("HS-R73", hs, 1, Some(7), ItemWord::Passed)];
    let got = report::method_line(&s.words, &items, &s.ws, &s.views, p, &s.r);
    assert_eq!(got, Err(WfErr::MethodItems), "Y3: an extra non-honest item printed {got:?}");
}

#[test]
fn rf1_y5_a_partition_missing_only_prov_universe_is_refused_by_name() {
    // the checker's surviving mutant Y5: every value declared but the universe (not ADANIENT), no RULES default
    let s = std_ctx();
    let mut r = s.r.clone();
    let views = Views::from_text(&views_text("STD"), &s.rows, &r, &[]).unwrap();
    r.partitions = partitions_of(&format!("id=1,underlying=X,instrument=cash,account=A1,universe=X,share_level=25%,exit_set=B,timeframe_family=minutes,{PROV_NO_UNIVERSE}")).unwrap();
    let got = check_paper_from(&r, &s.rows, &views);
    assert_eq!(got, Err(WfErr::NoPartitionProvenance { partition: 1, dim: "universe" }), "Y5: {got:?}");
    assert_eq!(got.unwrap_err().to_string(), "NoPartitionProvenance(p1,universe)");
}

#[test]
fn r1_no_honest_row_in_use_prints_a_not_passed_line_never_a_run_refusal() {
    // the checker's regression R1: period 2018-09..2022-12 holds neither WF nor TT wholly; v1-repaired refused the whole run
    // (E load: MethodItems) after its W08 lines
    let rules = RULES_NO_DEFAULT.replace("period=2018-09..2026-10", "period=2018-09..2022-12");
    let parts = format!("id=1,underlying=X,account=A1,universe=X,{PROV_ALL}");
    let case = Case { windows: "STD", views: "STD", cal: CAL_STD, rules: &rules, partitions: &parts, subjects: "", report: "", ops: "", host: "" };
    let out = run_case(&case, &windows_text, &views_text);
    assert!(!out.iter().any(|l| l.starts_with("E ")), "R1: the run was refused: {out:?}");
    let m: Vec<&String> = out.iter().filter(|l| l.starts_with("M p=1: ")).collect();
    assert_eq!(m.len(), 1, "R1: {out:?}");
    assert!(m[0].starts_with("M p=1: Both honest tests NOT PASSED (search: X -, -; account A1; universe X;") && m[0].ends_with("; WF-R73: NOT PASSED (not run); TT-R73: NOT PASSED (not run); same combo: no)"), "R1: {m:?}");
    // the API: no item -> the NOT PASSED line (one line, no W12b); an item of a row not in use -> refused
    let s = ctx("STD", &rules);
    assert!(s.ws.honest().is_empty());
    let p = &s.r.partitions[0];
    let got = report::method_line(&s.words, &[], &s.ws, &s.views, p, &s.r).unwrap().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(format!("M p=1: {}", got[0]), *m[0]);
    let wf = honest_rows(&s.rows)[0];
    assert_eq!(report::method_line(&s.words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed)], &s.ws, &s.views, p, &s.r), Err(WfErr::MethodItems));
}
