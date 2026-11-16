//! Property tests (G0-WF): L-INV, INC, MERGE (4 shards in all 24 orders), FEED-EQ, NEVER-KINDER, DD-NEVER-KINDER,
//! DD-NEVER-KINDER-MTM, PREFIX-EQ; the OB17 phrase rule over every vector line; the data-file refusals the vectors do not reach;
//! SHA-256 against the node reference's `crypto` digests; the W8 audit chain; the std-only guard; the red and legacy doors.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::print_stdout, clippy::similar_names, clippy::many_single_char_names, reason = "tests")]

use crate::date::{mk_of_day, parse_day};
use crate::rows::{self, Action, Audit, hex, sha256_bytes, verify_audit};
use crate::rules::{RunRules, check_paper_from};
use crate::tests_vec::{calendar_rows, partitions_of, read, rules_kv, vector_rows, views_text, windows_text, Case, run_case};
use crate::views::{Lists, Views};
use crate::windows::{WinRow, WindowSets, Windows, parse_windows};
use crate::*;

// repair of check-build-v1 B2: the partition's values take the run's signed default provenance (A2: never read so silently)
const RULES_STD: &str = "period=2018-09..2026-10&signed=2026-11&paper_from=2026-12&order=SEQ_LOSSES_FIRST&holes=lane&grid=1000:SEARCH:LATTICE&proof=proof step: test&data=data: complete&run=1&partition_provenance_default=RULED_FROM_TEXT property tests";
const CAL_STD: &str = "wd:2018-09-01:2026-10-31";
const PART: &str = "id=1,underlying=X";

struct Std {
    rules: RunRules,
    cal: Calendar,
    rows: Vec<WinRow>,
    views: Views,
    ws: WindowSets,
}
fn std_ctx(rules: &str, cal: &str, win: &str, views: &str) -> Std {
    let mut r = RunRules::from_rows(&rules_kv(rules)).unwrap();
    let cal = Calendar::from_rows(&calendar_rows(cal)).unwrap();
    let rows = parse_windows(&windows_text(win)).unwrap();
    let views = Views::from_text(&views_text(views), &rows, &r, &[]).unwrap();
    r.partitions = partitions_of(PART).unwrap();
    check_paper_from(&r, &rows, &views).unwrap();
    let ws = Windows::sets_from_rows(&rows, &cal, r.period).unwrap();
    Std { rules: r, cal, rows, views, ws }
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}
/// A random subject: trades on random walked sessions of the period, in order, holes allowed.
fn random_trades(g: &mut Lcg, cal: &Calendar, n: usize) -> Vec<Trade> {
    let d0 = parse_day("2018-09-03").unwrap();
    let d1 = parse_day("2026-10-30").unwrap();
    let mut days: Vec<i32> = (0..n).map(|_| d0 + i32::try_from(g.below(u64::try_from(d1 - d0).unwrap())).unwrap()).filter(|d| cal.walked(*d)).collect();
    days.sort_unstable();
    let mut out: Vec<Trade> = Vec::new();
    for (i, d) in days.iter().enumerate() {
        let net = i64::try_from(g.below(200_001)).unwrap() - 90_000;
        let close = u32::try_from(33_300 + g.below(22_000)).unwrap();
        out.push(Trade { day: *d, exit_day: *d, close_s: close, net, net_alt: 0, rd_bp: i32::try_from(g.below(500)).unwrap(), hole: g.below(10) == 0, id: u64::try_from(i).unwrap() + 1, mark_prev: 0 });
    }
    out.sort_by_key(|t| (t.exit_day, t.close_s, t.net));
    out
}
fn walk_trades(w: &Windows, s: &Std, trades: &[Trade]) -> Result<Acc, WfErr> {
    let mut a = Acc::new(w, &s.views, &s.rules);
    a.reset();
    for t in trades {
        a.add(t)?;
    }
    a.finish()?;
    Ok(a)
}
fn tickets(r: &RunRules) -> Tickets {
    let mut t = Tickets::new();
    t.push(Ticket { grid: 0, stage: NO_STAGE, set: ReadSet::EMPTY }, r).unwrap();
    t
}
/// Every split view's kept entries (code, key, learn summary, ties) of partition 1, as one comparable string.
fn split_lists(s: &Std, l: &Lists) -> String {
    let mut out = String::new();
    for (vi, v) in s.views.views.iter().enumerate() {
        let row = &s.rows[usize::from(v.win.0)];
        if row.kind != crate::windows::WinKind::Split && row.kind != crate::windows::WinKind::WayA {
            continue;
        }
        let id = ViewId(u16::try_from(vi).unwrap());
        out.push_str(&format!("{}:{}:{:?};", v.id, l.ties(Partition(1), id), l.counts(Partition(1), id)));
        for k in l.kept(Partition(1), id) {
            out.push_str(&format!("{:?}|{:?}|{:?},", k.key, k.learn, k.kind));
        }
        out.push('\n');
    }
    out
}
fn subjects(seed: u64, s: &Std, n: usize) -> Vec<(u64, Vec<Trade>)> {
    let mut g = Lcg(seed);
    (0..n).map(|i| { let k = 30 + usize::try_from(g.below(60)).unwrap(); (u64::try_from(i).unwrap() + 1, random_trades(&mut g, &s.cal, k)) }).collect()
}

#[test]
fn l_inv_test_months_never_move_a_split_pick() {
    // P20: change every test-month trade of every subject: every pick, key, tie count and list of every split and way-A view
    // stays byte-equal for the views whose learning part ends before the change (WF and the A rows by their own windows).
    let s = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    let subs = subjects(7, &s, 40);
    let wf_end = crate::date::parse_mk("2019-12").unwrap();
    let mut base = Lists::new(&s.views, w, &s.rules, 0);
    let mut scr = Lists::new(&s.views, w, &s.rules, 0);
    let mut g = Lcg(99);
    for (c, tr) in &subs {
        let a = walk_trades(w, &s, tr).unwrap();
        base.offer(&s.views, Partition(1), &tickets(&s.rules), Code([*c, 0, 0, 0]), &a).unwrap();
        let t2: Vec<Trade> = tr.iter().map(|t| if mk_of_day(t.exit_day) > wf_end { Trade { net: i64::try_from(g.below(300_001)).unwrap() - 150_000, hole: g.below(3) == 0, rd_bp: i32::try_from(g.below(900)).unwrap(), ..*t } } else { *t }).collect();
        let mut t2 = t2;
        t2.sort_by_key(|t| (t.exit_day, t.close_s, t.net));
        let b = walk_trades(w, &s, &t2).unwrap();
        scr.offer(&s.views, Partition(1), &tickets(&s.rules), Code([*c, 0, 0, 0]), &b).unwrap();
    }
    let only = |l: &Lists| -> String { split_lists(&s, l).lines().filter(|x| x.starts_with("WF-")).collect::<Vec<_>>().join("\n") };
    assert_eq!(only(&base), only(&scr), "L-INV: the WF views never read a test month");
    assert!(only(&base).contains("WF-R73"), "the WF views ran");
}

#[test]
fn inc_appended_months_change_no_split_pick() {
    // P24 INC: months 1..M, then the period grown by appended months (2026-11..2027-03, PAPER), equals for every split pick.
    let s1 = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let r2 = RULES_STD.replace("period=2018-09..2026-10", "period=2018-09..2027-03");
    let s2 = std_ctx(&r2, "wd:2018-09-01:2027-03-31", "STD", "STD");
    let subs = subjects(11, &s1, 30);
    let mut l1 = Lists::new(&s1.views, &s1.ws.sets[0], &s1.rules, 0);
    let mut l2 = Lists::new(&s2.views, &s2.ws.sets[0], &s2.rules, 0);
    let extra = Trade { day: parse_day("2027-01-04").unwrap(), exit_day: parse_day("2027-01-04").unwrap(), close_s: 40_000, net: -777_777, net_alt: 0, rd_bp: 0, hole: false, id: 9_999, mark_prev: 0 };
    for (c, tr) in &subs {
        let a = walk_trades(&s1.ws.sets[0], &s1, tr).unwrap();
        l1.offer(&s1.views, Partition(1), &tickets(&s1.rules), Code([*c, 0, 0, 0]), &a).unwrap();
        let mut t2 = tr.clone();
        t2.push(extra);
        let b = walk_trades(&s2.ws.sets[0], &s2, &t2).unwrap();
        l2.offer(&s2.views, Partition(1), &tickets(&s2.rules), Code([*c, 0, 0, 0]), &b).unwrap();
        // every split verdict on the honest rows is the same
        for h in s1.ws.honest() {
            let pick = PickWin { set: SetId(0), win: h, learn: s1.ws.sets[0].learn_range(h).unwrap(), kind: PickKind::Split, code: Code([*c, 0, 0, 0]) };
            let v1 = verdict(&s1.ws.sets[0], h, pick, &a.facts(Partition(1)), &s1.rules).unwrap();
            let v2 = verdict(&s2.ws.sets[0], h, pick, &b.facts(Partition(1)), &s2.rules).unwrap();
            assert_eq!(format!("{v1:?}"), format!("{v2:?}"), "INC: the verdict of {h:?}");
        }
    }
    let keys = |l: &Lists, s: &Std| -> String { split_lists(s, l).lines().filter(|x| x.starts_with("WF-") || x.starts_with("TT-")).map(|x| x.split(';').skip(1).collect::<Vec<_>>().join(";")).collect::<Vec<_>>().join("\n") };
    assert_eq!(keys(&l1, &s1), keys(&l2, &s2), "INC: every WF / TT pick and key is unchanged");
}

#[test]
fn merge_four_shards_in_all_24_orders() {
    // P13 MERGE (E3, V12): 4 shards merged in every order give the same bytes as one list.
    let s = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    let subs = subjects(3, &s, 48);
    let mut whole = Lists::new(&s.views, w, &s.rules, 0);
    let mut shards: Vec<Lists> = (0..4).map(|_| Lists::new(&s.views, w, &s.rules, 0)).collect();
    for (i, (c, tr)) in subs.iter().enumerate() {
        let a = walk_trades(w, &s, tr).unwrap();
        whole.offer(&s.views, Partition(1), &tickets(&s.rules), Code([*c, 0, 0, 0]), &a).unwrap();
        shards[i % 4].offer(&s.views, Partition(1), &tickets(&s.rules), Code([*c, 0, 0, 0]), &a).unwrap();
    }
    // three equal-key subjects across shards: the tie counter adds
    let tie = random_trades(&mut Lcg(5), &s.cal, 40);
    for k in 0..3u64 {
        let a = walk_trades(w, &s, &tie).unwrap();
        whole.offer(&s.views, Partition(1), &tickets(&s.rules), Code([1000 + k, 0, 0, 0]), &a).unwrap();
        shards[usize::try_from(k).unwrap()].offer(&s.views, Partition(1), &tickets(&s.rules), Code([1000 + k, 0, 0, 0]), &a).unwrap();
    }
    let want = split_lists(&s, &whole);
    let perms = [[0, 1, 2, 3], [0, 1, 3, 2], [0, 2, 1, 3], [0, 2, 3, 1], [0, 3, 1, 2], [0, 3, 2, 1], [1, 0, 2, 3], [1, 0, 3, 2], [1, 2, 0, 3], [1, 2, 3, 0], [1, 3, 0, 2], [1, 3, 2, 0], [2, 0, 1, 3], [2, 0, 3, 1], [2, 1, 0, 3], [2, 1, 3, 0], [2, 3, 0, 1], [2, 3, 1, 0], [3, 0, 1, 2], [3, 0, 2, 1], [3, 1, 0, 2], [3, 1, 2, 0], [3, 2, 0, 1], [3, 2, 1, 0]];
    for p in perms {
        let mut m = shards[p[0]].clone();
        for &i in &p[1..] {
            m.merge(&shards[i], &s.views).unwrap();
        }
        assert_eq!(split_lists(&s, &m), want, "MERGE order {p:?}");
    }
}

#[test]
fn feed_eq_trades_and_month_cells_agree_on_additive_and_class_fields() {
    // F9 FEED-EQ: the trades and their month cells give the same additive and class fields; a K5-K7 view refuses a month-fed Acc.
    let s = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    for (_, tr) in subjects(21, &s, 12) {
        let a = walk_trades(w, &s, &tr).unwrap();
        let mut b = Acc::new(w, &s.views, &s.rules);
        b.reset();
        let mut cells: Vec<MonthCell> = Vec::new();
        for t in &tr {
            let mk = mk_of_day(t.exit_day);
            if cells.last().is_none_or(|c| c.mk != mk) {
                cells.push(MonthCell { mk, net: 0, net_alt: 0, trades: 0, wins: 0, win_net: 0, loss_net: 0, hole_win_net: 0, holes: 0 });
            }
            let c = cells.last_mut().unwrap();
            c.net += t.net;
            c.trades += 1;
            if t.net > 0 {
                c.wins += 1;
                c.win_net += t.net;
            } else if t.net < 0 {
                c.loss_net += t.net;
            }
            if t.hole && t.net > 0 {
                c.hole_win_net += t.net;
                c.holes += 1;
            }
        }
        for c in &cells {
            b.add_month(c).unwrap();
        }
        b.finish().unwrap();
        let (fa, fb) = (a.facts(Partition(1)), b.facts(Partition(1)));
        assert_eq!((fa.booked, fa.second, fa.touched), (fb.booked, fb.second, fb.touched), "FEED-EQ masks");
        for (x, y) in fa.units.iter().zip(fb.units.iter()) {
            assert_eq!((x.net, x.trades, x.hole_win, x.hole_wins), (y.net, y.trades, y.hole_win, y.hole_wins), "FEED-EQ units");
        }
        for h in s.ws.honest() {
            let (la, lb) = (a.learn(h).unwrap(), b.learn(h).unwrap());
            let f = |c: &crate::acc::Cell| (c.net, c.trades, c.wins, c.win_net, c.loss_net, c.hole_win, c.m_traded, c.m_pos, c.m_zero, c.m_neg, c.m_worst, c.losing);
            assert_eq!(f(&la.cell), f(&lb.cell), "FEED-EQ learning cells");
            assert_eq!((la.counted, la.booked, la.touched, la.gap), (lb.counted, lb.booked, lb.touched, lb.gap));
        }
        let mut l = Lists::new(&s.views, w, &s.rules, 0);
        assert_eq!(l.offer(&s.views, Partition(1), &tickets(&s.rules), Code([1, 0, 0, 0]), &b), Err(WfErr::NeedsTrades), "a K5 view refuses a month-fed Acc");
    }
}

#[test]
fn never_kinder_second_reading_and_drawdown_bases() {
    // P7 NEVER-KINDER: second-positive units are booked-positive; P15 DD-NEVER-KINDER: the TRADE drawdown >= the SESSION one.
    let s = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    for (_, tr) in subjects(31, &s, 60) {
        let a = walk_trades(w, &s, &tr).unwrap();
        let f = a.facts(Partition(1));
        assert_eq!(f.second & !f.booked, 0, "NEVER-KINDER per unit");
        for v in s.views.views.iter().map(|v| v.win).collect::<std::collections::BTreeSet<_>>() {
            let l = a.learn(v).unwrap();
            assert!(l.cell.dd_trade.max_dd >= l.cell.dd_session.max_dd, "DD-NEVER-KINDER {v:?}");
        }
    }
}

#[test]
fn dd_never_kinder_with_marks_and_booked_figures_unchanged() {
    // R3 DD-NEVER-KINDER-MTM: with session-end marks, K5 = max(closed, mtm) >= the closed-trade K5, and every booked figure,
    // unit and verdict is byte-equal with and without the marks.
    let r = RULES_STD.to_owned() + "&marks=session_end";
    let s = std_ctx(&r, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    let mut g = Lcg(77);
    for _ in 0..40 {
        let tr = random_trades(&mut g, &s.cal, 40);
        let mut with = Acc::new(w, &s.views, &s.rules);
        with.reset();
        let mut last_day = 0;
        for t in &tr {
            if t.exit_day > last_day && last_day > 0 && g.below(3) == 0 {
                with.add_session_mtm(last_day, -i64::try_from(g.below(5_000_000)).unwrap()).unwrap();
            }
            with.add(t).unwrap();
            last_day = t.exit_day;
        }
        with.finish().unwrap();
        let without = walk_trades(w, &s, &tr).unwrap();
        assert_eq!(with.facts(Partition(1)).units, without.facts(Partition(1)).units, "booked units unchanged");
        assert_eq!(with.facts(Partition(1)).booked, without.facts(Partition(1)).booked);
        for h in s.ws.honest() {
            let (a, b) = (with.learn(h).unwrap(), without.learn(h).unwrap());
            assert_eq!((a.cell.net, a.cell.trades, a.cell.dd_trade), (b.cell.net, b.cell.trades, b.cell.dd_trade));
            let k5 = crate::views::crit_val(&crate::views::Crit::K5(crate::views::Base::Trade, None), &a, &with, s.rules.base_capital, true, &[]).unwrap();
            assert!(k5 >= b.cell.dd_trade.max_dd, "K5 with marks never kinder");
        }
    }
}

#[test]
fn prefix_eq_a_snapshot_equals_the_fold_of_its_pieces() {
    // B5 (b) PREFIX-EQ: an anchored row's one prefix cell equals the bounded fold over its learning pieces (the rolling path).
    let s = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    for (_, tr) in subjects(41, &s, 30) {
        let a = walk_trades(w, &s, &tr).unwrap();
        for v in s.views.views.iter().map(|v| v.win).collect::<std::collections::BTreeSet<_>>() {
            let spec = w.spec(v).unwrap();
            assert!(spec.anchored);
            assert_eq!(a.learn(v).unwrap().cell, a.fold_pieces(spec.learn_pieces).unwrap(), "PREFIX-EQ {v:?}");
        }
    }
}

#[test]
fn ob17_verdict_phrases_only_at_the_start_of_verdict_lines() {
    // Section 7 / OB17 (the crate half): across every vector's output, the four verdict phrases appear only at the start of a
    // line's text, and every such line matches a frozen W01-W09 / W26 / W27 form; no W05 / figures / luck / world line holds one.
    let phrases = ["Walk-forward test PASSED", "Walk-forward test NOT PASSED", "Train/test PASSED", "Train/test NOT PASSED"];
    let forms = ["PASSED (", "NOT PASSED (years not positive: ", "NOT PASSED (positive only through trades that met a true feed hole after their decision: ", "NOT PASSED (not run: "];
    let mut n = 0usize;
    for r in vector_rows() {
        let c = &r.cols;
        let case = Case { windows: &c[2], views: &c[3], cal: &c[4], rules: &c[5], partitions: &c[6], subjects: &c[7], report: &c[8], ops: &c[9], host: &c[10] };
        for line in run_case(&case, &windows_text, &views_text) {
            let text = line.split_once(": ").map_or(line.as_str(), |x| x.1);
            for p in phrases {
                if let Some(i) = text.find(p) {
                    assert_eq!(i, 0, "{}: a verdict phrase inside a line: {text}", r.id);
                    let rest = text.strip_prefix(if p.starts_with("Walk") { "Walk-forward test " } else { "Train/test " }).unwrap();
                    assert!(forms.iter().any(|f| rest.starts_with(f)), "{}: not a frozen form: {text}", r.id);
                    n += 1;
                }
            }
        }
    }
    println!("OB17: {n} verdict lines, every one at the start of its line and in a frozen form");
    assert!(n > 150);
}

#[test]
fn data_file_refusals_the_vectors_do_not_reach() {
    // P2: an unsorted calendar; 4.3 V-W1: a literal year in a template; a missing word id; 4.1: an unknown column.
    let a = CalRow { day: parse_day("2020-01-02").unwrap(), kind: DayKind::Regular, walked: true, ruling: false };
    let b = CalRow { day: parse_day("2020-01-01").unwrap(), ..a };
    assert_eq!(Calendar::from_rows(&[a, b]), Err(WfErr::BadRow { file: b'C', row: 2, rule: 1 }));
    assert_eq!(Calendar::from_rows(&[a, a]), Err(WfErr::BadRow { file: b'C', row: 2, rule: 1 }));
    let w = read("data/WORDS-STD-v1.tsv");
    assert_eq!(Words::from_text(&w.replace("{T} PASSED ({booked_only}", "{T} PASSED in 2024 ({booked_only}")), Err(WfErr::BadRow { file: b'T', row: 1, rule: 1 }));
    assert_eq!(Words::from_text(&w.replace("W27\t", "W99\t")), Err(WfErr::BadRow { file: b'T', row: 0, rule: 2 }));
    assert_eq!(parse_windows(&windows_text("STD").replacen("id\tkind", "id\tkinds", 1)), Err(WfErr::BadRow { file: b'W', row: 0, rule: 6 }));
    assert!(RunRules::from_rows(&rules_kv("period=2018-09..2026-10&signed=2026-11&paper_from=2026-12&bogus=1")).is_err());
}

#[test]
fn set_mismatch_double_offer_red_and_legacy_doors() {
    let s = std_ctx(RULES_STD, CAL_STD, "SETS2", "SETS2");
    assert!(s.ws.sets.len() >= 2);
    let a1 = walk_trades(&s.ws.sets[1], &s, &[]).unwrap();
    let mut l0 = Lists::new(&s.views, &s.ws.sets[0], &s.rules, 0);
    assert_eq!(l0.offer(&s.views, Partition(1), &tickets(&s.rules), Code([1, 0, 0, 0]), &a1), Err(WfErr::SetMismatch), "R5: another set's Acc");
    let f = a1.facts(Partition(1));
    let pick = PickWin { set: SetId(0), win: WinId(0), learn: (0, 0), kind: PickKind::Split, code: Code([1, 0, 0, 0]) };
    assert_eq!(verdict(&s.ws.sets[0], WinId(0), pick, &f, &s.rules), Err(WfErr::SetMismatch), "R5: another set's Facts");
    let a0 = walk_trades(&s.ws.sets[0], &s, &[]).unwrap();
    l0.offer(&s.views, Partition(1), &tickets(&s.rules), Code([2, 0, 0, 0]), &a0).unwrap();
    let again = l0.offer(&s.views, Partition(1), &tickets(&s.rules), Code([2, 0, 0, 0]), &a0);
    if cfg!(feature = "offer_once_check") {
        assert_eq!(again, Err(WfErr::DoubleOffer), "F3: a second offer of one code");
    } else {
        assert!(again.is_ok());
    }
    assert_eq!(refuse_if_red().is_err(), RED || MUTANT || cfg!(feature = "offer_once_check"));
    assert_eq!(refuse_if_legacy(Profile::Legacy("o5s")), Err(WfErr::LegacyHeadline));
    assert_eq!(refuse_if_legacy(Profile::Standard), Ok(()));
    assert!(Lists::bound_bytes(&s.views, 90, 8) > 0);
}

#[test]
fn learn_first_close_until_and_learn_ready() {
    // 5.1's learn-first host: close_until(learn_end + 1) makes the WF learning part readable before the test months are walked.
    let s = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    let wf = s.ws.honest()[0];
    let tr = random_trades(&mut Lcg(55), &s.cal, 80);
    let mut a = Acc::new(w, &s.views, &s.rules);
    a.reset();
    let learn_end = crate::date::parse_mk("2019-12").unwrap();
    for t in tr.iter().filter(|t| mk_of_day(t.exit_day) <= learn_end) {
        a.add(t).unwrap();
    }
    assert!(!a.learn_ready(wf));
    a.close_until(learn_end + 1).unwrap();
    assert!(a.learn_ready(wf));
    let early = a.learn(wf).unwrap();
    for t in tr.iter().filter(|t| mk_of_day(t.exit_day) > learn_end) {
        a.add(t).unwrap();
    }
    a.finish().unwrap();
    assert_eq!(early, a.learn(wf).unwrap(), "the learning part is fixed before any test month is walked");
}

#[test]
fn sha256_matches_the_node_reference() {
    let mut s = String::new();
    for i in 0..1000u32 {
        s.push(char::from(u8::try_from(32 + (i * 7) % 90).unwrap()));
    }
    assert_eq!(hex(&sha256_bytes(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(hex(&sha256_bytes(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(hex(&sha256_bytes(s.as_bytes())), "c57e9cddb50e2001754aa15df0d3c2db5e893e22731b5adc93f7302197515f77");
    assert_eq!(hex(&sha256_bytes(b"seq\tist")), "7a7486227d290b5f8d0b0f09912a48eed00d781c75ccf4f1b6e5ff5b4987995e");
}

#[test]
fn audit_chain_shows_any_edit() {
    let mut a = Audit::new("cwf-run-1", "builder-1", "binsha");
    a.append("2026-11-13 17:00:00 IST", Action::OpenRun, "-", "-", 0, 0, "open").unwrap();
    a.append("2026-11-13 17:00:01 IST", Action::WritePartition, "W3/p1.tsv", "abc", 10, 100, "-").unwrap();
    a.append("2026-11-13 17:00:02 IST", Action::CloseRun, "-", "-", 0, 0, "close").unwrap();
    let body = a.body();
    assert_eq!(verify_audit(&body), Ok(3));
    let edited = body.replace("W3/p1.tsv\tabc\t10", "W3/p1.tsv\tabc\t11");
    assert_eq!(verify_audit(&edited), Err(WfErr::BadRow { file: b'R', row: 3, rule: 21 }), "an edit of row 2 breaks row 3's link");
    assert!(rows::line(rows::W8, &["x".to_owned()]).is_err());
    let m = rows::manifest(&[("audit.tsv", &body)], "run");
    assert!(m.contains(&hex(&sha256_bytes(body.as_bytes()))));
    for t in [rows::W1, rows::W2, rows::W3, rows::W4, rows::W5, rows::W6, rows::W7, rows::W8, rows::W9] {
        let set: std::collections::BTreeSet<&&str> = t.iter().collect();
        assert_eq!(set.len(), t.len(), "unique column names");
    }
}

#[test]
fn std_only_no_dependency() {
    // as evalcore's `evalcore_builds_std_only`: [dependencies] is empty.
    let toml = read("Cargo.toml");
    let deps = toml.split("[dependencies]").nth(1).unwrap().split('[').next().unwrap();
    assert!(deps.lines().all(|l| l.trim().is_empty() || l.trim().starts_with('#')), "no dependency");
}

#[test]
fn luck_line_holds_no_verdict_phrase() {
    let s = std_ctx(RULES_STD, CAL_STD, "STD", "STD");
    let w = &s.ws.sets[0];
    let words = Words::from_text(&read("data/WORDS-STD-v1.tsv")).unwrap();
    let tr = random_trades(&mut Lcg(66), &s.cal, 120);
    let a = walk_trades(w, &s, &tr).unwrap();
    let wf = s.ws.honest()[0];
    let ctx = crate::report::LuckCtx { eligible: 5, offered: 9, views: 25, partitions: 1, twin: Some((3, 1)), mirror: Some(Some(false)) };
    let line = crate::report::luck_line(&words, w, wf, &tr, &a.facts(Partition(1)), &ctx).unwrap();
    assert!(line.starts_with("LUCK: the top 1 / 3 / 5 trades hold "));
    assert!(!line.contains("Walk-forward test PASSED") && !line.contains("Train/test PASSED") && !line.contains("NOT PASSED"));
    assert!(line.contains("twin (random entries, 3 seeds): the twin's pick had every test year positive in 1 of 3 seeds"));
    let _ = &s.rows;
}
