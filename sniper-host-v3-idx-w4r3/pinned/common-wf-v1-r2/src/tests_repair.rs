//! The repair round of `state/common-wf-20261113/check-build-v1.md` (2026-11-13). Each test was written RED FIRST against the
//! v1 API and failed on v1 (`repair-build-v1-work/RED-FIRST-run-red0.log`); after the repair only the item construction gained
//! the new `MethodItem` fields and the assertions were tightened to the exact refusals (never loosened). Repair 2
//! (check-build-v1-r2 B1-R) changed only the `method_line` call shape (the run's window sets and views in place of the honest
//! slice); its one slice-only case (a non-honest row passed as honest) is now impossible by type and reads the HS item instead:
//! - B1: `report::method_line` prints "Both honest tests PASSED" only over exactly one HEADLINE item per honest row of THIS
//!   search (REQ 73d; MAIN 14:31:36 IST: the same search's walk-forward pick PASSED and its train/test pick PASSED);
//! - B2: a REAL partition value with no declared provenance fails closed, by name (A2); ADANIENT alone carries MAIN's A2 ruling,
//!   RULED_KNOWINGLY(2026-04), so its W14K line is never left out (SNIPER-REQUIREMENTS s12);
//! - F2: a carried trade in a run whose RULES declare neither carried trades nor session-end marks is refused (P6, R3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, reason = "tests")]

use crate::date::parse_day;
use crate::report::{self, ItemWord, MethodItem};
use crate::rules::{RunRules, check_paper_from};
use crate::tests_vec::{calendar_rows, partitions_of, read, rules_kv, views_text, windows_text};
use crate::views::Views;
use crate::windows::{WinKind, WinRow, Windows, parse_windows};
use crate::*;

const RULES_NO_DEFAULT: &str = "period=2018-09..2026-10&signed=2026-11&paper_from=2026-12&order=SEQ_LOSSES_FIRST&holes=lane&grid=1000:SEARCH:LATTICE&proof=proof step: test&data=data: complete&run=1";
const PROV_ALL: &str = "prov_account=RULED_FROM_TEXT t,prov_universe=RULED_FROM_TEXT t,prov_share_level=RULED_FROM_TEXT t,prov_exit_set=RULED_FROM_TEXT t,prov_timeframe_family=RULED_FROM_TEXT t,prov_side_setting=RULED_FROM_TEXT t,prov_fill_level=RULED_FROM_TEXT t,prov_timing=RULED_FROM_TEXT t,prov_reading=RULED_FROM_TEXT t";
const PROV_NO_UNIVERSE: &str = "prov_account=RULED_FROM_TEXT t,prov_share_level=RULED_FROM_TEXT t,prov_exit_set=RULED_FROM_TEXT t,prov_timeframe_family=RULED_FROM_TEXT t,prov_side_setting=RULED_FROM_TEXT t,prov_fill_level=RULED_FROM_TEXT t,prov_timing=RULED_FROM_TEXT t,prov_reading=RULED_FROM_TEXT t";

fn honest_rows(rows: &[WinRow]) -> Vec<&WinRow> {
    rows.iter().filter(|r| r.kind == WinKind::Split && r.honest).collect()
}
fn item(view: &str, row: &WinRow, part: u32, code: Option<u64>, word: ItemWord) -> MethodItem {
    MethodItem { win: WinId(row.idx), partition: Partition(part), view: view.to_owned(), label: row.pick_label.clone(), code: code.map(|c| Code([c, 0, 0, 0])), word }
}

#[test]
fn b1_method_line_needs_one_headline_item_per_honest_row_of_this_search() {
    let words = Words::from_text(&read("data/WORDS-STD-v1.tsv")).unwrap();
    let rows = parse_windows(&windows_text("STD")).unwrap();
    let h = honest_rows(&rows);
    assert_eq!(h.len(), 2);
    let (wf, tt) = (h[0], h[1]);
    let mut r = RunRules::from_rows(&rules_kv(RULES_NO_DEFAULT)).unwrap();
    r.partitions = partitions_of(&format!("id=1,underlying=X,account=A1,universe=X,{PROV_ALL}")).unwrap();
    let p = r.partitions[0].clone();
    // repair 2 (check-build-v1-r2 B1-R): method_line takes the run's window sets and views and reads the honest rows itself
    let cal = Calendar::from_rows(&calendar_rows("wd:2018-09-01:2026-10-31")).unwrap();
    let views = Views::from_text(&views_text("STD"), &rows, &r, &[]).unwrap();
    let ws = Windows::sets_from_rows(&rows, &cal, r.period).unwrap();
    let both = [item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("TT-R73", tt, 1, Some(7), ItemWord::Passed)];
    let ok = report::method_line(&words, &both, &ws, &views, &p, &r).unwrap().unwrap();
    assert!(ok[0].starts_with("Both honest tests PASSED (") && ok[0].ends_with("WF-R73 WALK-FORWARD PICK 7: PASSED; TT-R73 TRAIN/TEST PICK 7: PASSED; same combo: yes)"), "{ok:?}");
    // the checker's reproducer: ONE PASSED walk-forward item for two honest rows
    let one = report::method_line(&words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed)], &ws, &views, &p, &r);
    assert_eq!(one, Err(WfErr::MethodItems), "B1: one item for two honest rows printed {one:?}");
    // two walk-forward items, no train/test item
    let two_wf = report::method_line(&words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("WF-R73", wf, 1, Some(7), ItemWord::Passed)], &ws, &views, &p, &r);
    assert_eq!(two_wf, Err(WfErr::MethodItems), "B1: two walk-forward items printed {two_wf:?}");
    // no item at all
    let none = report::method_line(&words, &[], &ws, &views, &p, &r);
    assert_eq!(none, Err(WfErr::MethodItems), "B1: no item printed {none:?}");
    // a PASSED word with no pick code; a "no pick" item with a code
    let nocode = report::method_line(&words, &[item("WF-R73", wf, 1, None, ItemWord::Passed), item("TT-R73", tt, 1, Some(7), ItemWord::Passed)], &ws, &views, &p, &r);
    assert_eq!(nocode, Err(WfErr::MethodItems), "B1: a PASSED item with no code printed {nocode:?}");
    let nopick_code = report::method_line(&words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("TT-R73", tt, 1, Some(7), ItemWord::NoPick)], &ws, &views, &p, &r);
    assert_eq!(nopick_code, Err(WfErr::MethodItems));
    // the caller's order never moves the line: the items print in the honest rows' file order
    let swapped = report::method_line(&words, &[both[1].clone(), both[0].clone()], &ws, &views, &p, &r).unwrap().unwrap();
    assert_eq!(swapped, ok, "B1: the line depends on the caller's item order");
    // never across searches: the train/test item of partition 2 in partition 1's line
    let across = report::method_line(&words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("TT-R73", tt, 2, Some(7), ItemWord::Passed)], &ws, &views, &p, &r);
    assert_eq!(across, Err(WfErr::SearchMismatch), "B1: a method line across searches printed {across:?}");
    // an item of a row that is not an honest row, and a non-honest row passed as an honest one
    let hs = rows.iter().find(|x| x.id == "HS").unwrap();
    let foreign = report::method_line(&words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("HS-R73", hs, 1, Some(7), ItemWord::Passed)], &ws, &views, &p, &r);
    assert_eq!(foreign, Err(WfErr::MethodItems));
    // (repair 2: a non-honest row can no longer be passed AS an honest row, the rows are the run's own; the same refusal holds
    // for the HS row's item named by the walk-forward HEADLINE view's id)
    let not_honest = report::method_line(&words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("WF-R73", hs, 1, Some(7), ItemWord::Passed)], &ws, &views, &p, &r);
    assert_eq!(not_honest, Err(WfErr::MethodItems));
    // each word apart when only one test passed: NOT PASSED, the PASSED word and the NOT PASSED word both printed
    let split = report::method_line(&words, &[item("WF-R73", wf, 1, Some(7), ItemWord::Passed), item("TT-R73", tt, 1, Some(9), ItemWord::NotPassed)], &ws, &views, &p, &r).unwrap().unwrap();
    assert!(split[0].starts_with("Both honest tests NOT PASSED (") && split[0].ends_with("WF-R73 WALK-FORWARD PICK 7: PASSED; TT-R73 TRAIN/TEST PICK 9: NOT PASSED; same combo: no)"), "{split:?}");
    // the print door is fail closed too: a hand-built partition with no provenance and no default is refused here
    let undeclared = partitions_of("id=1,underlying=X,account=A1").unwrap();
    assert_eq!(report::method_line(&words, &both, &ws, &views, &undeclared[0], &r), Err(WfErr::NoPartitionProvenance { partition: 1, dim: "account" }));
}

#[test]
fn b2_partition_provenance_fails_closed_and_adanient_carries_w14k() {
    let words = Words::from_text(&read("data/WORDS-STD-v1.tsv")).unwrap();
    let rows = parse_windows(&windows_text("STD")).unwrap();
    let h = honest_rows(&rows);
    let mut r = RunRules::from_rows(&rules_kv(RULES_NO_DEFAULT)).unwrap();
    let views = Views::from_text(&views_text("STD"), &rows, &r, &[]).unwrap();
    // (a) the checker's reproducer: a REAL partition with values and no declared provenance, no signed default
    r.partitions = partitions_of("id=1,underlying=X,account=A1,universe=X").unwrap();
    let got = check_paper_from(&r, &rows, &views);
    assert_eq!(got, Err(WfErr::NoPartitionProvenance { partition: 1, dim: "account" }), "B2: a partition with no declared provenance: {got:?}");
    assert_eq!(got.unwrap_err().to_string(), "NoPartitionProvenance(p1,account)");
    // (b) ADANIENT alone with no prov_universe: MAIN's A2 ruling still gives the W14K line (both universe spellings, and an
    // ADANIENT underlying with no universe value)
    for spec in ["underlying=ADANIENT,universe=ADANIENT", "underlying=ADANIENT,universe=ADANIENT alone", "underlying=ADANIENT spot"] {
        let parts = partitions_of(&format!("id=1,{spec},{PROV_NO_UNIVERSE}")).unwrap();
        let k = report::knowingly_lines(&words, &parts[0], &h).unwrap();
        assert_eq!(k.len(), 1, "B2: {spec}: no W14K line: {k:?}");
        assert!(k[0].starts_with("the instrument and its pattern were chosen from the operator's chart of 2026-04 (a test month of both windows): design-level hindsight"), "{k:?}");
    }
    // not ADANIENT alone: no ruling
    for spec in ["underlying=ADANIENT,universe=F&O stocks", "underlying=TCS,universe=TCS", "underlying=ADANIENT,universe=ADANIENT and TCS"] {
        let parts = partitions_of(&format!("id=1,{spec},{PROV_ALL}")).unwrap();
        assert!(report::knowingly_lines(&words, &parts[0], &h).unwrap().is_empty(), "{spec}");
    }
    // (c) ADANIENT alone signed RULED_FROM_TEXT, or another chart month, contradicts MAIN's ruling: refused by name; the ruling
    // itself declared explicitly is accepted
    let bad = partitions_of(&format!("id=1,underlying=ADANIENT,universe=ADANIENT,{PROV_ALL}"));
    assert_eq!(bad, Err(WfErr::RulingContradicted { partition: 1, dim: "universe" }), "B2: ADANIENT alone read as RULED_FROM_TEXT");
    assert_eq!(partitions_of(&format!("id=1,underlying=ADANIENT,universe=ADANIENT,{PROV_NO_UNIVERSE},prov_universe=RULED_KNOWINGLY(2026-05)")), Err(WfErr::RulingContradicted { partition: 1, dim: "universe" }));
    assert!(partitions_of(&format!("id=1,underlying=ADANIENT,universe=ADANIENT,{PROV_NO_UNIVERSE},prov_universe=RULED_KNOWINGLY(2026-04)")).is_ok());
    // (d) control: every value declared -> accepted
    r.partitions = partitions_of(&format!("id=1,underlying=X,account=A1,universe=X,{PROV_ALL}")).unwrap();
    assert_eq!(check_paper_from(&r, &rows, &views), Ok(()));
    // (e) the signed RULES default covers every undeclared value; it must be RULED_FROM_TEXT with a signed line
    let mut rd = RunRules::from_rows(&rules_kv(&format!("{RULES_NO_DEFAULT}&partition_provenance_default=RULED_FROM_TEXT MAIN 2026-11-13 line"))).unwrap();
    assert_eq!(rd.partition_prov_default.as_deref(), Some("MAIN 2026-11-13 line"));
    rd.partitions = partitions_of("id=1,underlying=X,account=A1,universe=X").unwrap();
    assert_eq!(check_paper_from(&rd, &rows, &views), Ok(()));
    for v in ["RULED_FROM_TEXT", "RULED_FROM_TEXT  ", "CHOSEN(2020-01..2026-10; 4)", "RULED_KNOWINGLY(2026-04)", "LATTICE"] {
        assert_eq!(RunRules::from_rows(&rules_kv(&format!("{RULES_NO_DEFAULT}&partition_provenance_default={v}"))), Err(WfErr::BadRow { file: b'R', row: 0, rule: 2 }), "{v}");
    }
    // (f) a COMPARISON partition (by role or named in RULES) needs no provenance; a twin world partition (role REAL) does
    r.partitions = partitions_of("id=1,underlying=X,account=A1,role=COMPARISON").unwrap();
    assert_eq!(check_paper_from(&r, &rows, &views), Ok(()));
    let mut rc = RunRules::from_rows(&rules_kv(&format!("{RULES_NO_DEFAULT}&comparison=2"))).unwrap();
    rc.partitions = partitions_of(&format!("id=1,underlying=X,{PROV_ALL}|id=2,account=A2")).unwrap();
    assert_eq!(check_paper_from(&rc, &rows, &views), Ok(()));
    r.partitions = partitions_of(&format!("id=1,underlying=X,{PROV_ALL}|id=2,world=TWIN seed 3")).unwrap();
    assert_eq!(check_paper_from(&r, &rows, &views), Err(WfErr::NoPartitionProvenance { partition: 2, dim: "side_setting" }));
}

#[test]
fn f2_a_carried_trade_needs_rules_that_declare_carried_trades() {
    let cal = Calendar::from_rows(&calendar_rows("wd:2018-09-01:2026-10-31")).unwrap();
    let rows = parse_windows(&windows_text("STD")).unwrap();
    let t = Trade { day: parse_day("2019-01-02").unwrap(), exit_day: parse_day("2019-01-03").unwrap(), close_s: 36_000, net: 100, net_alt: 0, rd_bp: 0, hole: false, id: 1, mark_prev: 0 };
    for (extra, want_ok) in [("", false), ("&carried=yes", true), ("&marks=session_end", true)] {
        let mut r = RunRules::from_rows(&rules_kv(&format!("{RULES_NO_DEFAULT}{extra}"))).unwrap();
        let views = Views::from_text(&views_text("STD"), &rows, &r, &[]).unwrap();
        r.partitions = partitions_of(&format!("id=1,underlying=X,account=A1,universe=X,{PROV_ALL}")).unwrap();
        let ws = Windows::sets_from_rows(&rows, &cal, r.period).unwrap();
        let mut a = Acc::new(&ws.sets[0], &views, &r);
        a.reset();
        let got = a.add(&t);
        assert_eq!(got.is_ok(), want_ok, "F2 rules '{extra}': {got:?}");
        if !want_ok {
            assert_eq!(got, Err(WfErr::CarriedUndeclared));
        }
        // an intraday trade is booked under every RULES
        let mut b = Acc::new(&ws.sets[0], &views, &r);
        b.reset();
        assert!(b.add(&Trade { exit_day: t.day, ..t }).is_ok());
    }
}

#[test]
fn f7_a_finished_run_audit_ends_at_its_one_close_run() {
    use crate::rows::{Action, Audit, verify_audit, verify_closed_audit};
    let mut a = Audit::new("cwf-run-1", "repair-1", "binsha");
    a.append("2026-11-13 18:00:00 IST", Action::OpenRun, "-", "-", 0, 0, "open").unwrap();
    for i in 0..3u64 {
        a.append("2026-11-13 18:00:01 IST", Action::WritePartition, &format!("W3/p{i}.tsv"), "abc", 10, 100, "-").unwrap();
    }
    let open_body = a.body();
    a.append("2026-11-13 18:00:02 IST", Action::CloseRun, "-", "-", 0, 0, "close").unwrap();
    let body = a.body();
    assert_eq!(verify_audit(&body), Ok(5));
    assert_eq!(verify_closed_audit(&body), Ok(5));
    // the checker's probe: rows 1-4 of 5 (the CLOSE_RUN cut away) still chain, so verify_audit alone accepts them (v1: Ok(4));
    // the finished-run check refuses the cut tail
    let cut: String = body.lines().take(5).map(|l| format!("{l}\n")).collect();
    assert_eq!(cut, open_body);
    assert_eq!(verify_audit(&cut), Ok(4));
    assert_eq!(verify_closed_audit(&cut), Err(WfErr::BadRow { file: b'R', row: 4, rule: 22 }));
    // a header only; a CLOSE_RUN that is not the last row
    assert_eq!(verify_closed_audit(&format!("{}\n", crate::rows::header(crate::rows::W8))), Err(WfErr::BadRow { file: b'R', row: 0, rule: 22 }));
    let mut b = Audit::new("cwf-run-2", "repair-1", "binsha");
    b.append("2026-11-13 18:00:00 IST", Action::OpenRun, "-", "-", 0, 0, "open").unwrap();
    b.append("2026-11-13 18:00:01 IST", Action::CloseRun, "-", "-", 0, 0, "close").unwrap();
    b.append("2026-11-13 18:00:02 IST", Action::WritePartition, "W3/late.tsv", "abc", 1, 1, "-").unwrap();
    assert_eq!(verify_closed_audit(&b.body()), Err(WfErr::BadRow { file: b'R', row: 2, rule: 22 }));
    // an edited row is still the chain's refusal
    assert_eq!(verify_closed_audit(&body.replace("W3/p1.tsv\tabc\t10", "W3/p1.tsv\tabc\t11")), Err(WfErr::BadRow { file: b'R', row: 4, rule: 21 }));
}

#[test]
fn f12_k17_money_gap_value_on_a_miss_row() {
    // K17 MONEY_GAP = the learning units' money gap to +1 paisa (F9) + one per untouched learning unit; asserted as a key value
    // of a WF-MISS entry (the checker's follow-up F12: K17 was asserted only through ordering)
    let cal = Calendar::from_rows(&calendar_rows("wd:2018-09-01:2026-10-31")).unwrap();
    let rows = parse_windows(&windows_text("STD")).unwrap();
    let mut r = RunRules::from_rows(&rules_kv(&format!("{RULES_NO_DEFAULT}&partition_provenance_default=RULED_FROM_TEXT t"))).unwrap();
    let views = Views::from_text(&views_text("STD"), &rows, &r, &[]).unwrap();
    r.partitions = partitions_of("id=1,underlying=X").unwrap();
    let ws = Windows::sets_from_rows(&rows, &cal, r.period).unwrap();
    let w = &ws.sets[0];
    let tr = |d: &str, net: i64, id: u64| Trade { day: parse_day(d).unwrap(), exit_day: parse_day(d).unwrap(), close_s: 36_000, net, net_alt: 0, rd_bp: 0, hole: false, id, mark_prev: 0 };
    let mut a = Acc::new(w, &views, &r);
    a.reset();
    // 2018 (Sep-Dec) one trade at -Rs 5.00: gap 501 paisa; 2019 no trade: untouched; then test trades
    for t in [tr("2018-09-03", -500, 1), tr("2020-01-02", 1_000, 2), tr("2021-01-04", 1_000, 3)] {
        a.add(&t).unwrap();
    }
    a.finish().unwrap();
    let mut tk = Tickets::new();
    tk.push(Ticket { grid: 0, stage: NO_STAGE, set: ReadSet::EMPTY }, &r).unwrap();
    let mut l = Lists::new(&views, w, &r, 0);
    l.offer(&views, Partition(1), &tk, Code([5, 0, 0, 0]), &a).unwrap();
    let vi = views.views.iter().position(|v| v.id == "WF-MISS").unwrap();
    let kept = l.kept(Partition(1), ViewId(u16::try_from(vi).unwrap()));
    assert_eq!(kept.len(), 1);
    let k = &kept[0].key;
    let names: Vec<&str> = views.views[vi].key.iter().take(3).map(crate::views::Crit::name).collect();
    assert_eq!(names, ["K1 YEARS_NOT_POS", "K16 TRADE_GAP", "K17 MONEY_GAP"]);
    assert_eq!(k.parts.get(..3), Some(&[2i64, 1, 502][..]), "K1 = 2 learning units not positive, K16 = 1 untouched, K17 = 501 + 1");
}
