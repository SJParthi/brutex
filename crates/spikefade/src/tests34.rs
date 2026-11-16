//! v3.4 round D build:D-high NAMED TESTS (MAIN-RULINGS-ENGINE-v22 Addendum 30: every HIGH item of the v3.3 gates and re-check that can change
//! a result, plus the quick result-neutral ones). One test (or more) per switch of `fix34.rs`; every case is synthetic and integer; each named
//! mutant (Cargo features `m_v34_*`) must make its test FAIL. The switches are per test thread; every test starts with all off.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_precision_loss)]

use crate::acct::{SEv, Sum};
use crate::cal::{NM, at_sod, days_from_civil, ymd_of};
use crate::cube::{Contract, Cube, Und};
use crate::fix34 as f;
use crate::kern::{Acc, CSet, CapDay, Pieces, Pos};
use crate::stage::{ABSENT, EvX, NR, Tab3};
use crate::sw;
use crate::trade::{ACCOUNT, BASE_S, EvT, NL, Out, Refuse};

fn reset() {
    for s in 0..sw::COUNT {
        sw::set(s, false);
    }
    for s in 0..crate::sw33::COUNT {
        crate::sw33::set(s, false);
    }
    for s in 0..crate::fix33::COUNT {
        crate::fix33::set(s, false);
    }
    for s in 0..crate::fixg::COUNT {
        crate::fixg::set(s, false);
    }
    for s in 0..f::COUNT {
        f::set(s, false);
    }
    f::TEST_HALTS.with(|t| *t.borrow_mut() = None);
    f::TEST_ELM.with(|t| *t.borrow_mut() = None);
    f::TEST_STAMP.with(|t| *t.borrow_mut() = None);
    f::TEST_BANDS.with(|t| *t.borrow_mut() = None);
    f::TEST_EVENTS.with(|t| *t.borrow_mut() = None);
}

fn off(s: i64) -> u16 {
    u16::try_from(s - BASE_S).unwrap()
}

/// A tiny deterministic generator (tests only).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + i64::try_from(self.next() % u64::try_from(hi - lo + 1).unwrap()).unwrap()
    }
}

// ------------------------------------------------------------------------------------------------ switches (P0 words)

/// v3.4: every switch is OFF by default (proof P0: v3.3 final's code paths and prints): no words, no fingerprint part, one Tops group set,
/// the checkpoint lines without the v3.4 fields; `--fix34` turns on exactly the switches that need no MAIN ruling (BANDS needs data:F7's
/// table, the DIALS a value) and refuses to start without their tables; a dial is on only with its own value.
#[test]
fn v34_switches_default_off_fix34_all() {
    reset();
    f::from_args(&["run3".to_owned()]).unwrap();
    assert!(!f::any(), "no flag: every v3.4 switch off");
    assert_eq!(f::words(), "");
    assert_eq!(f::fp_part(), "", "the checkpoint fingerprint stays v3.3's");
    assert_eq!(f::cap_groups(), 1);
    let sm = Sum { trades: 3, net: 7, cap_first1: 0, ..Sum::default() };
    assert_eq!(crate::ckpt::sum_line(&sm).split('\t').count(), 29, "v3.3's 29 summary fields, no v3.4 field");
    assert_eq!(crate::ckpt::fig_line(&crate::wf::Fig::default()).split('\t').count(), 10);
    // --fix34 without the tables: refused by name (never pretends)
    assert!(f::from_args(&["run3".to_owned(), "--fix34".to_owned()]).is_err());
    let dir = std::env::temp_dir().join(format!("sf34-sw-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("h.tsv"), "date\tkind\tfrom\tresume\tsource\n2020-03-13\thalt\t09:22:59\t10:21:00\tt\n").unwrap();
    std::fs::write(dir.join("e.tsv"), "session\tsymbol\trate_h\tfirst_ael_oth_h\n2018-10-01\tSAIL\t731\t-1\n").unwrap();
    std::fs::write(dir.join("s.tsv"), "from\tto\tbuy_paise_per_crore\tboth_paise_per_crore\tclass\tsource\n2018-01-01\t2020-06-30\t0\t200000\tB\tx\n").unwrap();
    let a: Vec<String> = ["run3", "--fix34", "--halts", dir.join("h.tsv").to_str().unwrap(), "--elm-pre-ael", dir.join("e.tsv").to_str().unwrap(), "--stamp-dated", dir.join("s.tsv").to_str().unwrap()].iter().map(|x| (*x).to_owned()).collect();
    assert!(f::from_args(&a).is_err(), "CAP20 needs the running MTM marks and the exact margin");
    sw::set(sw::MTM, true);
    sw::set(sw::MARGIN_EXACT, true);
    f::from_args(&a).unwrap();
    for s in 0..f::COUNT {
        assert_eq!(f::on(s), f::FIX34_ALL.contains(&s), "--fix34 sets switch {s} iff it needs no ruling");
    }
    assert!(!f::on(f::BANDS) && !f::on(f::RISKCAP) && !f::on(f::TAILGATE));
    assert!(!f::fp_part().is_empty() && f::fp_part().contains("Rs 20 L cap at EVERY trading minute"));
    assert_eq!(f::cap_groups(), 2);
    for x in ["h.tsv", "e.tsv", "s.tsv"] {
        let _ = std::fs::remove_file(dir.join(x));
    }
    let _ = std::fs::remove_dir(&dir);
    reset();
}

/// v3.4 CAP20: the account state, the position and the year piece keep their v3.3 sizes (the new fields sit in padding), so the day-major
/// batch gate and every memory projection are unchanged.
#[test]
fn v34_cap20_state_sizes_unchanged() {
    assert_eq!(std::mem::size_of::<Acc>(), 232, "Acc (v3.3: 232 B)");
    assert_eq!(std::mem::size_of::<Pos>(), 48, "Pos (v3.3: 48 B)");
    assert_eq!(std::mem::size_of::<crate::kern::Piece>(), 64, "Piece (v3.3: 64 B)");
}

// ------------------------------------------------------------------------------------------------ CAP20 (matrix X20)

/// One CAP20 fixture event: decision second, margin of its one lot (paisa), units, value sold, exit second, net, the contract-day's marks.
struct CapEv {
    t: u32,
    margin: i64,
    units: u32,
    sold: i64,
    exit: u32,
    net: i32,
    marks: Vec<i32>,
}

/// The kernel fixture of one session (run day 0): one event per contract, rule 0 (the SL) firing at T + 10 s into out slot 1 (that event's
/// position: units, value sold, exit, net), the contract-day's marks and (CAP20) their suffix maxima.
fn cap_fixture(v: &[CapEv]) -> (Vec<SEv>, Vec<EvX>, Tab3) {
    let mut evs = Vec::new();
    let mut ex = Vec::new();
    let mut tab = Tab3::default();
    for (k, e) in v.iter().enumerate() {
        let k32 = u32::try_from(k).unwrap();
        let et = EvT { refuse: Refuse::None, lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot: e.margin, filled: [true; NL], ..EvT::default() };
        let mark1 = u32::try_from(tab.marks.len()).unwrap() + 1;
        let mut mk = e.marks.clone();
        mk.resize(NM, *e.marks.last().unwrap_or(&0));
        if f::on(f::CAP20) {
            tab.smax.extend(f::suffix_max(&mk));
        }
        tab.marks.extend(mk);
        let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap(), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1 };
        for _l in 0..NL {
            let mut a = [ABSENT; NR];
            a[0] = off(i64::from(e.t) + 10);
            let mut o = [0_u8; NR];
            o[0] = 1;
            tab.anc.push(a);
            tab.os.push(o);
            tab.obase.push(u32::try_from(tab.outs.len()).unwrap());
            tab.outs.push(Out::default());
            tab.outs.push(Out { net: e.net, exit: off(i64::from(e.exit)), flag: 1, units: e.units, sold: e.sold, ..Out::default() });
        }
        evs.push(SEv { di: 0, ci: k32, t: e.t, skey: k32, tab: et });
        ex.push(x);
    }
    (evs, ex, tab)
}

/// The REFERENCE per-minute rule (addendum 17; `v32::margin_mtm_peak`'s rule): for every trading minute b of the session, the sum over the
/// BOOKED positions open in b (from the decision's minute to the minute holding the last fill) of margin + max(0, mark(b) x units - value
/// sold); the minutes above Rs 20 L counted. The booked positions are read from the kernel's own detailed decisions.
fn cap_reference(evs: &[SEv], ex: &[EvX], tab: &Tab3, cs: &CSet, ni: usize) -> u16 {
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let (dec, _, _) = crate::kern::detail3(evs, ex, tab, &list, cs, 0, ni, None, true, 1);
    let mut pos: Vec<(u16, u16, i64, u32, u32, i64)> = Vec::new();
    // the same positions for v3.3's OWN post-hoc per-minute checker (fix33::cap_minutes, its flat Rs 20 L count): an independent second reading
    let day = days_from_civil(2026, 11, 4);
    let mut t5: Vec<(i64, i64, i64, i64, i64, crate::v32::PMark)> = Vec::new();
    for d in dec.iter().filter(|d| d.code == "booked") {
        let (e, x) = (&evs[d.eid as usize], &ex[d.eid as usize]);
        let o = crate::kern::out_of_decision(e, x, tab, cs, 0, ni, None).unwrap();
        let exit_sod = u32::try_from(i64::from(o.exit) + BASE_S).unwrap();
        pos.push((crate::kern::cap_minute(e.t), crate::kern::cap_end(exit_sod), i64::from(d.lots) * e.tab.margin_lot, x.mark1, o.units, o.sold));
        t5.push((at_sod(day, i64::from(e.t)), at_sod(day, i64::from(exit_sod)), i64::from(d.lots) * e.tab.margin_lot, i64::from(o.net), day, crate::v32::PMark { units: u64::from(o.units), sold: o.sold, mark1: x.mark1 }));
    }
    let v33_flat = crate::fix33::cap_minutes(&t5, &tab.marks, &[day]).above_flat;
    let mut n = 0_u16;
    for b in 0..crate::kern::CAP_NM {
        let mut v = 0_i64;
        for &(i0, i1, m, mark1, units, sold) in &pos {
            if i0 <= b && b < i1 {
                let mk = i64::from(tab.marks[mark1 as usize - 1 + usize::from(b)]);
                v += m + (mk * i64::from(units) - sold).max(0);
            }
        }
        if v > ACCOUNT {
            n += 1;
        }
    }
    assert_eq!(u32::from(n), v33_flat, "the reference equals v3.3's own per-minute checker (fix33::cap_minutes, flat Rs 20 L)");
    n
}

fn cs_sl() -> CSet {
    CSet { rules: vec![(false, 0)], sl: 0 }
}

/// The kernel's count for one account walked whole (`walk_account`, exact skip on or off): (the piece's minutes, the summary's first day).
fn cap_kernel(evs: &[SEv], ex: &[EvX], tab: &Tab3, ni: usize, skip: bool) -> (u16, u16) {
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = crate::kern::list_keys(evs, &list);
    let (sm, pc, _) = crate::kern::walk_account(evs, ex, tab, &list, &keys, &cs_sl(), 0, ni, None, true, skip, &[0], crate::wf::NPIECE, 1);
    (pc.p[0].cap, sm.cap_first1)
}

/// MATRIX X20 (standing F6 remainder; round C's `v32c_cap_minutes_equal_margin_mtm_peak`, ported): the kernel counts EVERY trading minute
/// whose margin in use + running MTM loss (minute-HIGH marks) passes Rs 20 L, exactly as the reference rule, on 400 random sessions and on the
/// three edge cases the O(1) bound and the partial minute must get right: (a) a breach only in the minute a position closes in (its share kept
/// after the flush: mutant m_v34_cap_drop_closed fails), (b) a breach from a LATE re-spike of a position booked early (the bound reads the
/// highest LATER mark: m_v34_cap_bound_low fails), (c) any breach at all (m_v34_cap_skip_always fails). Skip on and off agree; with the
/// switch off nothing is counted.
#[test]
fn v34_cap20_kernel_counts_every_minute_equal_reference() {
    reset();
    f::set(f::CAP20, true);
    let ni = 3; // 40 slots
    let flat = |p: i32| vec![p; NM];
    // (a) A: Rs 15 L margin, 100 units sold at Rs 100.00 (Rs 10,000), its mark jumps to Rs 3,100.00 in minute 75 (10:30) and A closes at
    // 10:30:20; C is booked at 10:30:40 (minute 75) with Rs 4 L: minute 75 = 15 L + 100 x 3,100 - 10,000 (Rs 3 L) + 4 L = Rs 22 L
    let mut mk_a = flat(10_000);
    mk_a[75] = 310_000;
    let a = vec![
        CapEv { t: 36_000, margin: 150_000_000, units: 100, sold: 1_000_000, exit: 37_820, net: 5_000, marks: mk_a },
        CapEv { t: 37_840, margin: 40_000_000, units: 100, sold: 1_000_000, exit: 39_000, net: 5_000, marks: flat(10_000) },
    ];
    // (b) B: Rs 18 L margin booked 09:20, flat until minute 200 (12:35) where its mark makes a Rs 3 L loss; the bound at its FIRST minute is 0
    let mut mk_b = flat(10_000);
    mk_b[200] = 310_000;
    let b = vec![CapEv { t: 33_600, margin: 180_000_000, units: 100, sold: 1_000_000, exit: 50_000, net: -1_000, marks: mk_b }];
    for (name, fx, want) in [("closing minute", &a, 1_u16), ("late re-spike", &b, 1_u16)] {
        let (evs, ex, tab) = cap_fixture(fx);
        assert_eq!(cap_reference(&evs, &ex, &tab, &cs_sl(), ni), want, "{name}: the reference");
        for skip in [true, false] {
            let (n, first) = cap_kernel(&evs, &ex, &tab, ni, skip);
            assert_eq!(n, want, "{name} (skip {skip}): the kernel counts the reference's minutes");
            assert_eq!(first, 1, "{name}: the summary names run day 0 (first + 1)");
        }
    }
    // random sessions near the cap
    let mut r = Rng(0x5eed_cafe);
    let mut breached = 0;
    for trial in 0..400 {
        let n = usize::try_from(r.range(2, 9)).unwrap();
        let mut v = Vec::new();
        let mut t = 33_300_i64;
        for _ in 0..n {
            t += r.range(0, 1_800);
            let p = i32::try_from(r.range(2_000, 30_000)).unwrap();
            let mut mk = flat(p);
            for _ in 0..r.range(0, 3) {
                let m = usize::try_from(r.range(0, 374)).unwrap();
                mk[m] = p * i32::try_from(r.range(1, 9)).unwrap();
            }
            let units = u32::try_from(r.range(25, 1_000)).unwrap();
            let dur = r.range(20, 14_000);
            v.push(CapEv { t: u32::try_from(t).unwrap(), margin: r.range(20_000_000, 70_000_000), units, sold: i64::from(units) * i64::from(p), exit: u32::try_from((t + dur).min(55_800)).unwrap(), net: i32::try_from(r.range(-40_000, 60_000)).unwrap(), marks: mk });
        }
        let (evs, ex, tab) = cap_fixture(&v);
        let want = cap_reference(&evs, &ex, &tab, &cs_sl(), ni);
        breached += usize::from(want > 0);
        for skip in [true, false] {
            assert_eq!(cap_kernel(&evs, &ex, &tab, ni, skip).0, want, "trial {trial} skip {skip}");
        }
    }
    assert!(breached > 40, "the random set holds breaching sessions ({breached} of 400)");
    // switch off: nothing counted (v3.3)
    f::set(f::CAP20, false);
    let (evs, ex, tab) = cap_fixture(&a);
    assert_eq!(cap_kernel(&evs, &ex, &tab, ni, true), (0, 0));
    reset();
}

/// MATRIX X20 with D2-1 (the v3.3 day-class ghost walk): the ghost trace carries the day's minutes above the cap (they read only the day's
/// positions) and every replayed class member gets them; the replay equals the member's own walk (`walk_day_h`). Mutant
/// m_v34_cap_ghost_dropped (the count not carried) fails.
#[test]
fn v34_cap20_ghost_replay_carries_count() {
    reset();
    f::set(f::CAP20, true);
    let ni = 3;
    let mut mk = vec![10_000; NM];
    mk[120] = 310_000;
    let v = vec![
        CapEv { t: 36_000, margin: 120_000_000, units: 100, sold: 1_000_000, exit: 45_000, net: 3_000, marks: mk },
        CapEv { t: 36_600, margin: 55_000_000, units: 50, sold: 500_000, exit: 44_000, net: 2_000, marks: vec![10_000; NM] },
    ];
    let (evs, ex, tab) = cap_fixture(&v);
    let want = cap_reference(&evs, &ex, &tab, &cs_sl(), ni);
    assert!(want > 0);
    let sub: Vec<u32> = vec![0, 1];
    let subk = crate::kern::list_keys(&evs, &sub);
    let of_day = [0_u8];
    // the member's own walk
    let mut ac = Acc::new();
    let mut pc = Pieces::new(&of_day, crate::wf::NPIECE);
    let mut buf = vec![Pos::default(); crate::kern::MAXCAP];
    let _ = crate::kern::walk_day_h(&evs, &ex, &tab, &sub, &subk, &cs_sl(), 0, ni, None, true, true, &mut ac, &mut pc, &of_day, &mut buf);
    assert_eq!(pc.p[0].cap, want, "the walk counts the reference");
    // the ghost and the replay
    let mut g = crate::kern::Trace::default();
    crate::kern::ghost_day(&evs, &ex, &tab, &sub, &subk, &cs_sl(), 0, ni, None, true, &of_day, &mut buf, &mut g);
    assert_eq!(g.cap_day, want, "the trace carries the day's count");
    let mut ac2 = Acc::new();
    let mut pc2 = Pieces::new(&of_day, crate::wf::NPIECE);
    assert!(crate::kern::replay_day(&mut ac2, &mut pc2, &g, 0, 0, &mut buf, crate::kern::cap_at(0, ni)));
    assert_eq!(pc2.p[0].cap, pc.p[0].cap, "the replay marks the piece as the walk");
    assert_eq!((ac2.cap_first1, ac2.cap_last1), (ac.cap_first1, ac.cap_last1), "and the account's first / last day");
    let _ = CapDay::new();
    reset();
}

// ------------------------------------------------------------------------------------------------ CAP20 views, windows, splits

/// MATRIX X20 (round C's `v32c_cap_keeps_breaching_accounts_out`, ported to every view): an account with a trading minute above Rs 20 L is
/// kept out of every view (m_v34_cap_view_ignored fails); a window row never picks a candidate whose LEARNING pieces hold one
/// (m_v34_cap_learn_ignored fails) while one in its TEST pieces only is picked and reads NOT PASSED (no look-ahead: the learning years
/// could not see it); the WIN candidates read the kept AND the excluded groups (m_v34_cap_cands_kept_only fails). Off: nothing changes.
#[test]
fn v34_cap20_views_windows_candidates() {
    reset();
    let bad = Sum { trades: 5, net: 9_000_000, cap_first1: 3, cap_last1: 3, ..Sum::default() };
    let good = Sum { trades: 5, net: 1_000_000, ..Sum::default() };
    assert!(!f::cap_view_out(&bad), "switch off: every account in the views (v3.3)");
    f::set(f::CAP20, true);
    assert!(f::cap_view_out(&bad) && !f::cap_view_out(&good), "an account with a breach minute is kept out of every view");
    // windows: row learn pieces 0..1, test pieces 2..3
    let row = crate::wf::WinRow { id: "T".to_owned(), learn: (0, 1), test: (2, 3), growing: true };
    let mk = |cap_learn: u16, cap_test: u16| -> Vec<crate::kern::Piece> {
        let mut p = vec![crate::kern::Piece { net: 1_000, trades: 2, ..crate::kern::Piece::default() }; crate::wf::NPIECE];
        p[1].cap = cap_learn;
        p[3].cap = cap_test;
        p
    };
    let mut win = vec![crate::report3::WinRes { row: Some(row.clone()), run: true, ..crate::report3::WinRes::default() }];
    crate::report3::offer_windows(&mut win, std::slice::from_ref(&row), &mk(2, 0), Sum { seg: 0, trades: 4, ..Sum::default() }, None);
    assert!(win[0].best.is_none(), "a breach in a learning year: never the row's pick");
    crate::report3::offer_windows(&mut win, std::slice::from_ref(&row), &mk(0, 1), Sum { seg: 0, trades: 4, ..Sum::default() }, None);
    let b = win[0].best.as_ref().expect("a breach in a test year only: picked (the learning years could not see it)");
    assert!(f::cap_test_failed(b.3.cap), "and the row reads NOT PASSED");
    // the WIN candidates: kept and excluded groups both
    let mut t1 = crate::acct::Tops::new(5);
    t1.offer(good);
    let mut t2 = crate::acct::Tops::new(5);
    t2.offer(bad);
    assert_eq!(f::cand_tops(&[t1.clone()], &[t2.clone()]).len(), 2, "the candidates read both group sets");
    reset();
}

/// MATRIX X20 on the Addendum 18 splits (every split's learning window starts at the run's first month, its test runs to the last): a combo
/// whose LEARNING months hold a minute above Rs 20 L is never a fold's pick (m_v34_cap_split_learn_ignored fails); one whose breach falls
/// in the test months is picked and FAILS every split whose test months hold it (its per-combo pass counts drop). Off: v3.3.
#[test]
fn v34_cap20_splits_learning_refused_test_failed() {
    reset();
    sw::set(sw::SPLITS18, true);
    let d0 = days_from_civil(2018, 10, 1);
    let days: Vec<i64> = (d0..d0 + 31 * 41).filter(|&d| crate::rules::weekday(d) < 5 && crate::v32::month_id(d) - crate::v32::month_id(d0) < 40).collect();
    crate::split18::set_cal(crate::split18::SCal::from_days(&days).unwrap());
    let c = crate::split18::cal().unwrap();
    // one close a session of Rs 20,000: every month >= Rs 2 L, every week piece and session positive (passes every split without the cap)
    let ser = || {
        let mut s = crate::split18::Ser18 { s: crate::split18::SSeries::new(c), m: None };
        let mut real = 0_i64;
        for d in 0..days.len() {
            real += 2_000_000;
            s.close(u32::try_from(d).unwrap() * 86_400 + 40_000, real, 2_000_000, true);
        }
        s
    };
    let day_of_month = |m: usize| u16::try_from(c.mo_of_day.iter().position(|&x| usize::from(x) == m).unwrap()).unwrap();
    let run = |sm0: Sum| -> (crate::split18::SpStat, crate::split18::Sel18) {
        let mut sel = crate::split18::Sel18::new();
        let mut sm = sm0;
        crate::split18::account(Some(&ser()), &mut sm, &mut sel, true, (0, 0, 0, 0, 0));
        (sm.sp, sel)
    };
    let tt19 = |sel: &crate::split18::Sel18| sel.splits.iter().position(|s| s.id == "TT-2019").unwrap();
    let clean = Sum { trades: 600, net: 1, ..Sum::default() };
    let (st0, sel0) = run(clean);
    assert!(sel0.best[tt19(&sel0)][0].is_some() && st0.pass19 == st0.run, "without a breach: picked, every split passes");
    f::set(f::CAP20, true);
    // a breach in month 3 (2019-01): inside TT-2019's learning months (2018-10..2019-12)
    let learn_bad = Sum { cap_first1: day_of_month(3) + 1, cap_last1: day_of_month(3) + 1, ..clean };
    let (_, sel1) = run(learn_bad);
    assert!(sel1.best[tt19(&sel1)][0].is_none(), "a breach in the learning months: never the fold's pick");
    // a breach in month 20 (2020-06): learning clean (picked), every split testing 2020-06 fails
    let test_bad = Sum { cap_first1: day_of_month(20) + 1, cap_last1: day_of_month(20) + 1, ..clean };
    let (st2, sel2) = run(test_bad);
    assert!(sel2.best[tt19(&sel2)][0].is_some(), "a breach after the learning months: picked (no look-ahead)");
    assert!(st2.pass19 < st0.pass19 && st2.run == st0.run, "and the splits whose test months hold it fail ({} of {} pass)", st2.pass19, st2.run);
    assert!(f::cap_in_months(f::cap_months(&test_bad, &c.mo_of_day), 15, c.m() - 1), "TT-2019's way B test window holds it: NOT PASSED");
    reset();
}

// ------------------------------------------------------------------------------------------------ HALTS (standing F17, exits F-H3, X09)

const HALTS_V1: &str = "date\tkind\tfrom\tresume\tsource\n2020-03-13\thalt\t09:22:59\t10:21:00\tt\n2020-03-23\thalt\t09:58:38\t10:57:00\tt\n2021-02-24\toutage\t11:40:00\t-\tt\n2024-03-02\tshort\t12:30:00\t-\tt\n";

/// STANDING F17 / EXITS F-H3 / MATRIX X09 (`sf22_halt_segments_2020_03_13_and_23`, `oob_halt_segments_from_dated_table`): the dated table cuts
/// 2020-03-13 and 2020-03-23 into TWO tape segments at the store's own halt (no block spans it: the crate's segment walk resets the carried
/// entitlement there and a stop decided across it buys back at the first post-halt block's worst: crate test `v31_carry_resets_at_a_halt`),
/// a decision inside the halt is refused (`halted`), the first post-halt print reads the second before the resumption as its previous print
/// (as 09:15's first print reads 09:14:59), 2021-02-24 (outage, no resumed session held) and 2024-03-02 (ends before the 15:10 square-off)
/// are refused by name; an ordinary session and the switch off give v3.3's one segment. Mutants m_v34_one_segment and
/// m_v34_halt_decisions_taken fail.
#[test]
fn v34_halt_segments_2020_03_13_and_23() {
    reset();
    let rows = f::HaltTable::parse(HALTS_V1).unwrap();
    f::TEST_HALTS.with(|t| *t.borrow_mut() = Some(f::HaltTable { rows, src: "test".to_owned() }));
    let d13 = days_from_civil(2020, 3, 13);
    let open = at_sod(d13, crate::cal::OPEN_S);
    let end = at_sod(d13, 15 * 3_600 + 40 * 60);
    assert_eq!(f::segments_of(d13, open, end).len(), 1, "switch off: one segment (v3.3)");
    f::set(f::HALTS, true);
    let s = f::segments_of(d13, open, end);
    assert_eq!(s.len(), 2, "the halt cuts the session");
    assert_eq!((s[0].open, s[0].last_sec), (open, at_sod(d13, 9 * 3_600 + 22 * 60 + 58)), "the first segment ends at the last pre-halt second");
    assert_eq!((s[1].open, s[1].last_sec), (at_sod(d13, 10 * 3_600 + 21 * 60), end - common_fill::US_S), "the second opens at the resumption");
    assert_eq!(f::segments_of(days_from_civil(2020, 3, 23), at_sod(days_from_civil(2020, 3, 23), crate::cal::OPEN_S), at_sod(days_from_civil(2020, 3, 23), 56_400)).len(), 2);
    assert_eq!(f::segments_of(days_from_civil(2020, 3, 12), at_sod(days_from_civil(2020, 3, 12), crate::cal::OPEN_S), at_sod(days_from_civil(2020, 3, 12), 56_400)).len(), 1, "an ordinary session");
    assert!(f::in_halt(d13, 10 * 3_600), "10:00:00 is inside the halt: refused");
    assert!(!f::in_halt(d13, 10 * 3_600 + 21 * 60) && !f::in_halt(d13, 9 * 3_600 + 22 * 60 + 58), "outside it: decided");
    // the first post-halt print's previous print is clamped to the second before the resumption
    let r = |sod: i64| common_fill::SecIn { ts: at_sod(d13, sod), high: 100, low: 100, vol: 10 };
    let rows2 = vec![r(9 * 3_600 + 22 * 60 + 58), r(10 * 3_600 + 21 * 60), r(10 * 3_600 + 21 * 60 + 1)];
    let mut prev = vec![at_sod(d13, 9 * 3_600 + 22 * 60 + 57), at_sod(d13, 9 * 3_600 + 22 * 60 + 58), at_sod(d13, 10 * 3_600 + 21 * 60)];
    f::clamp_prev_to_segments(&mut prev, &rows2, &s);
    assert_eq!(prev[1], at_sod(d13, 10 * 3_600 + 21 * 60) - common_fill::US_S, "the post-halt first print reads 10:20:59, never the pre-halt print");
    assert_eq!(prev[2], at_sod(d13, 10 * 3_600 + 21 * 60), "later prints unchanged");
    assert_eq!(f::session_refusal(days_from_civil(2021, 2, 24)), Some(Refuse::SessionOutage));
    assert_eq!(f::session_refusal(days_from_civil(2024, 3, 2)), Some(Refuse::ShortSession));
    assert_eq!(f::session_refusal(d13), None);
    // a trade whose walk crossed the halt is labelled with it; one after it is not
    assert!(f::halt_trade_words(d13, 9 * 3_600 + 20 * 60, 10 * 3_600 + 30 * 60).contains("crossed the halt 09:22:59-10:21:00"));
    assert_eq!(f::halt_trade_words(d13, 10 * 3_600 + 30 * 60, 11 * 3_600), "-");
    assert!(f::HaltTable::parse("date\tkind\tfrom\tresume\tsource\n2020-03-13\thalt\t10:00:00\t09:00:00\tt\n").is_err(), "a halt that resumes before it starts is refused");
    reset();
}

// ------------------------------------------------------------------------------------------------ LOT0 (standing F18)

fn idx_cube(day: i64, cons: Vec<(u16, u32, Vec<u32>)>, unds: &[&str]) -> Cube {
    let mut c = Cube { ymd: ymd_of(day), day, ..Cube::default() };
    for u in unds {
        c.unds.push(Und { name: (*u).to_owned(), kind: 0, spot_seg: 0, spot_sid: 1, nearest: 2026_11_10, next: 0 });
    }
    let n = cons.len();
    c.vol = vec![0; n * NM];
    c.flags = vec![0; n * NM];
    c.high = vec![0; n * NM];
    c.low = vec![0; n * NM];
    c.close = vec![0; n * NM];
    for (ci, (und, lot, vols)) in cons.into_iter().enumerate() {
        c.cons.push(Contract { sid: u32::try_from(ci).unwrap() + 1, und, kind: 0, right: 0, expiry: 2026_11_10, lot, strike: 2_400_000 + i64::try_from(ci).unwrap() * 5_000, first_min: 0, first_open: 1_000, symbol: format!("X{ci}"), n_sec: 0 });
        for (m, v) in vols.into_iter().enumerate() {
            c.vol[Cube::ix(ci, m)] = v;
            c.flags[Cube::ix(ci, m)] = 1;
        }
    }
    c
}

/// STANDING F18 (`oob_no_lot0_traded_contract_and_lot0_counted`): an INDEX contract at cube lot 0 takes its underlying's lot of the day when
/// that one lot divides every traded minute of the lot-0 contracts (exact: every trade a whole number of real lots); a lot-0 contract whose
/// quantities that lot does not divide, and a day whose index contracts are ALL at lot 0 (2019-05-30), stay out of the universe, counted
/// and named NO DATA (lot 0). Off: untouched. Mutant m_v34_lot0_any_lot fails.
#[test]
fn v34_lot0_resolved_or_named() {
    reset();
    let d = days_from_civil(2019, 5, 29);
    let mk = || idx_cube(d, vec![(0, 75, vec![150, 75]), (0, 0, vec![150, 300, 75]), (1, 20, vec![40]), (1, 0, vec![50, 30])], &["NIFTY", "BANKNIFTY"]);
    let mut c = mk();
    f::lot0_resolve(&mut c);
    assert_eq!(c.cons[1].lot, 0, "switch off: untouched (v3.3)");
    f::set(f::LOT0, true);
    let mut c = mk();
    f::lot0_resolve(&mut c);
    assert_eq!(c.cons[1].lot, 75, "NIFTY's lot-0 contract trades in whole lots of 75: resolved");
    assert_eq!(c.cons[3].lot, 0, "BANKNIFTY's lot-0 quantities (GCD 10) are not whole lots of 20: stays out, named");
    let mut all0 = idx_cube(days_from_civil(2019, 5, 30), vec![(0, 0, vec![75, 150]), (0, 0, vec![75])], &["NIFTY"]);
    f::lot0_resolve(&mut all0);
    assert!(all0.cons.iter().all(|k| k.lot == 0), "a day whose index contracts are all at lot 0: no lot to take, NO DATA");
    let w = f::lot0_words();
    assert!(w.contains("2019-05-29 NIFTY: 1 at lot 0 -> lot 75") && w.contains("2019-05-29 BANKNIFTY: 1 contracts NO DATA") && w.contains("2019-05-30 NIFTY: 2 contracts NO DATA"), "{w}");
    reset();
}

// ------------------------------------------------------------------------------------------------ ELMPRE (standing F28)

/// STANDING F28 (Addendum 4; `v33f_stock_elm_pre_ael_rule`): before NSE's first AEL file a stock's exposure comes from the dated rule table
/// (the higher of 5% and 1.5 sd of the daily log returns over the previous six months, both NSE's month-end basis and a rolling one: the
/// larger); a rate below the 5% floor is refused by the reader; off or no row = v3.3 (`no_elm_rate`). Mutant m_v34_elm_pre_floor_only fails.
#[test]
fn v34_stock_elm_pre_ael_rule() {
    reset();
    let (rows, last) = f::ElmTable::parse("session\tsymbol\trate_h\tfirst_ael_oth_h\n2018-10-01\tSAIL\t731\t-1\n2018-10-01\tINFY\t500\t-1\n2019-06-13\tSAIL\t688\t1040\n").unwrap();
    assert_eq!(last, 2019_06_13);
    f::TEST_ELM.with(|t| *t.borrow_mut() = Some(f::ElmTable { rows, last, src: "test".to_owned() }));
    assert_eq!(f::elm_pre(2018_10_01, "SAIL"), None, "switch off: v3.3 (the stock refused no_elm_rate)");
    f::set(f::ELMPRE, true);
    assert_eq!(f::elm_pre(2018_10_01, "SAIL"), Some(731), "1.5 sd above the 5% floor: 7.31%");
    assert_eq!(f::elm_pre(2018_10_01, "INFY"), Some(500), "the 5% floor");
    assert_eq!(f::elm_pre(2018_10_02, "SAIL"), None, "no row: v3.3 (refused, counted)");
    assert!(f::elm_pre_words(2018_10_01, "SAIL").starts_with("exposure from NSE's rule, no AEL file"), "Addendum 4's label on every such trade");
    assert!(f::ElmTable::parse("session\tsymbol\trate_h\tfirst_ael_oth_h\n2018-10-01\tSAIL\t499\t-1\n").is_err(), "below the 5% floor: refused");
    reset();
}

// ------------------------------------------------------------------------------------------------ STAMP (standing F30)

/// STANDING F30 (`v33f_stamp_pre_20200701_sourced_or_labelled`): the dated stamp table drives the charges (here a test table with 0.01% on
/// both sides before 2020-07-01); a session the table does not cover keeps v3.3's rows; the shipped table v34/tables-high/STAMP-DATED-v1.tsv
/// holds exactly v3.3's numbers (0.02% both sides = 200,000 paise a crore before 2020-07-01; 0.003% buy = 30,000 from it), so switching it on
/// changes no rupee until a sourced row replaces the bound. Mutant m_v34_stamp_table_ignored fails.
#[test]
fn v34_stamp_dated_table() {
    reset();
    let d19 = days_from_civil(2019, 6, 3);
    let d21 = days_from_civil(2021, 6, 3);
    let v33 = |d: i64| {
        let r = crate::charges::rates_on(d).unwrap();
        (r.stamp_buy, r.stamp_both)
    };
    assert_eq!(v33(d19), (0, 200_000), "v3.3: the conservative 0.02% both sides");
    assert_eq!(v33(d21), (30_000, 0), "v3.3: 0.003% on the buy side");
    let t = f::StampTable::parse("from\tto\tbuy_paise_per_crore\tboth_paise_per_crore\tclass\tsource\n2018-01-01\t2020-06-30\t0\t100000\tTEST\tt\n").unwrap();
    f::TEST_STAMP.with(|x| *x.borrow_mut() = Some(f::StampTable { rows: t, src: "test".to_owned() }));
    f::set(f::STAMP, true);
    assert_eq!(v33(d19), (0, 100_000), "the dated row drives the charges");
    assert_eq!(v33(d21), (30_000, 0), "a session the table does not cover: v3.3's row");
    // the shipped table's two rows
    let shipped = f::StampTable::parse("from\tto\tbuy_paise_per_crore\tboth_paise_per_crore\tclass\tsource\n2018-01-01\t2020-06-30\t0\t200000\tB\ts\n2020-07-01\t2099-12-31\t30000\t0\tS\ts\n").unwrap();
    f::TEST_STAMP.with(|x| *x.borrow_mut() = Some(f::StampTable { rows: shipped, src: "shipped".to_owned() }));
    for d in [days_from_civil(2018, 10, 1), d19, days_from_civil(2020, 6, 30), days_from_civil(2020, 7, 1), d21, days_from_civil(2026, 11, 4)] {
        let on = v33(d);
        f::set(f::STAMP, false);
        assert_eq!(on, v33(d), "the shipped table equals v3.3 on {d}");
        f::set(f::STAMP, true);
    }
    assert!(f::StampTable::parse("from\tto\tbuy_paise_per_crore\tboth_paise_per_crore\tclass\tsource\n2018-01-01\t2020-06-30\t0\t1\tB\ts\n2020-06-01\t2099-12-31\t0\t1\tS\ts\n").is_err(), "overlapping rows refused");
    reset();
}

// ------------------------------------------------------------------------------------------------ BANDS (exits F-H2, matrix X10)

/// EXITS F-H2 / MATRIX X10 (`xm_band_lock_zero_share`): a buy-back tape printing AT the upper band Rs 150.00 for 30 s (10,000 units a second),
/// then at Rs 149.00: with the band on the tape (crate v2.5 B1, every re-sent limit clipped one tick inside it) the order fills nothing while
/// the price sits at the band and fills at the first block below it, at that block's highest HIGH; without a band it fills AT the band at
/// once (v3.3). The engine's lookup puts the table's band on the contract-day's tape (m_v34_band_ignored fails).
#[test]
fn v34_band_lock_zero_share() {
    reset();
    sw::set(sw::RESEND, true);
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let mut rows = vec![crate::cube::Sec { sod: u32::try_from(t - 1).unwrap(), high: 14_000, low: 14_000, vol: 10_000 }];
    for s in 0..30 {
        rows.push(crate::cube::Sec { sod: u32::try_from(t + s).unwrap(), high: 15_000, low: 15_000, vol: 10_000 });
    }
    for s in 30..60 {
        rows.push(crate::cube::Sec { sod: u32::try_from(t + s).unwrap(), high: 14_900, low: 14_800, vol: 10_000 });
    }
    let dt = crate::trade::DayTape::new(d, &rows, 2).unwrap();
    let dl = crate::trade::dial().unwrap();
    let walk = |band: Option<(i64, i64, i64)>| -> common_fill::LevelFill {
        let tape = dt.tape().unwrap().with_band(band);
        let o = common_fill::Order { side: common_fill::Side::Buy, anchor: at_sod(d, t), stop: at_sod(d, crate::cal::SQ_S), unit: 50, limit: Some(common_fill::std70::band_clip(tape.band(), common_fill::Side::Buy, 16_500)), kind: common_fill::Kind::SignalExit };
        let mut want = [500_u64; common_fill::MAXL];
        let mut out = [common_fill::LevelFill::default(); common_fill::MAXL];
        common_fill::walk(&tape, &o, &dl, common_fill::LevelMask::all(NL), &mut want, None, None, &mut out, None).unwrap();
        out[1]
    };
    let free = walk(None);
    assert_eq!(free.first.map(|b| b.px), Some(15_000), "no band: the buy fills AT the band price at once (v3.3)");
    let locked = walk(Some((1_000, 15_000, 5)));
    let fb = locked.first.expect("it fills below the band");
    assert_eq!(fb.px, 14_900, "the first block below the band, at its highest HIGH");
    assert!(fb.block.at >= at_sod(d, t + 30), "nothing filled while the price sat at the band");
    // the engine's lookup: the table's row on the contract-day
    let mut c = idx_cube(d, vec![(0, 75, vec![75])], &["NIFTY"]);
    c.cons[0].strike = 2_400_000;
    let mut bt = f::BandTable::default();
    bt.rows.insert((ymd_of(d), "NIFTY".to_owned(), 2026_11_10, 2_400_000, 0), (1_000, 15_000));
    f::TEST_BANDS.with(|x| *x.borrow_mut() = Some(bt));
    assert_eq!(f::band3(&c, 0), None, "switch off: no band on the tape");
    f::set(f::BANDS, true);
    assert_eq!(f::band3(&c, 0), Some((1_000, 15_000, crate::trade::TICK)), "the table's band rides on the tape");
    reset();
}

// ------------------------------------------------------------------------------------------------ BANONCE (standing F24)

/// STANDING F24 = RECHECK-V33 F2: an F&O-ban refusal is counted ONCE per event (session, contract, decision second) however often it is
/// priced (a day-major run prices it once per batch whose lists hold it; a resume prices it again). Mutant m_v34_ban_per_pricing fails.
#[test]
fn v34_ban_counted_once_per_event() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let dir = std::env::temp_dir().join(format!("sf34-ban-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("fo_secban_04112026.csv"), "Securities in Ban For Trade Date 04-NOV-2026:\n1,SAIL\n").unwrap();
    std::fs::write(dir.join("fo_secban_05112026.csv"), "Securities in Ban For Trade Date 05-NOV-2026:\n").unwrap();
    let mut b = crate::ban::Bans::default();
    b.add_dir(&dir).unwrap();
    let _ = crate::ban::BANS.set(b);
    crate::ban::reset34();
    f::set(f::BANONCE, true);
    for _ in 0..3 {
        assert!(crate::ban::refuse_k(d, "SAIL", 7 * 1_048_576 + 36_000));
    }
    for _ in 0..2 {
        assert!(crate::ban::refuse_k(d, "SAIL", 7 * 1_048_576 + 36_060));
    }
    assert!(!crate::ban::refuse_k(d + 1, "SAIL", 7 * 1_048_576 + 36_000), "a NIL list the next session");
    assert_eq!(crate::ban::refused_events(), 2, "two events, five pricings: counted twice");
    for x in ["fo_secban_04112026.csv", "fo_secban_05112026.csv"] {
        let _ = std::fs::remove_file(dir.join(x));
    }
    let _ = std::fs::remove_dir(&dir);
    reset();
}

// ------------------------------------------------------------------------------------------------ RUNID / DURABLE / MEMGUARD (F8 / F11 / F12)

/// STANDING F8 (`run_id_unique_same_second_two_segments`, `oob_two_engines_same_out_refused`): two runs of the same second differ by their
/// segments and argv hash; a run folder is created exclusively (a second engine on the same folder is refused) with a LOCK file. Mutant
/// m_v34_runid_dir_shared fails.
#[test]
fn v34_run_id_unique_and_folder_exclusive() {
    reset();
    let a1: Vec<String> = ["run3", "c", "o", "--segs", "idx"].iter().map(|x| (*x).to_owned()).collect();
    let a2: Vec<String> = ["run3", "c", "o", "--segs", "stk"].iter().map(|x| (*x).to_owned()).collect();
    let i1 = f::unique_id("sf3-20261116-110000-v32", &[0], &a1);
    let i2 = f::unique_id("sf3-20261116-110000-v32", &[1], &a2);
    assert_ne!(i1, i2, "the same second, two segments: two ids");
    assert!(i1.starts_with("sf3-20261116-110000-v32-idx-p") && i2.contains("-stk-p"));
    assert_ne!(f::argv_hash(&a1), f::argv_hash(&a2));
    let dir = std::env::temp_dir().join(format!("sf34-runid-{}-{:?}", std::process::id(), std::thread::current().id()));
    let r = dir.join("runs-v3").join(&i1);
    f::create_run_dir(&r).unwrap();
    assert!(r.join("LOCK").exists(), "the LOCK file holds the pid");
    assert!(f::create_run_dir(&r).is_err(), "a second engine on the same folder is refused");
    let _ = std::fs::remove_file(r.join("LOCK"));
    let _ = std::fs::remove_dir(&r);
    let _ = std::fs::remove_dir(dir.join("runs-v3"));
    let _ = std::fs::remove_dir(&dir);
    reset();
}

/// STANDING F11 (`oob_ckpt_torn_write_falls_back_to_prev`, `ckpt_save_durable_falls_back_to_previous`): a durable checkpoint carries a
/// SHA-256 line, the previous generation is kept as `.ckpt.prev`, a torn newest file falls back to it, both torn = refused by name; result files
/// are written as `.part` and renamed (no reader sees half a file). Off: v3.3's write. Mutants m_v34_ckpt_no_prev and
/// m_v34_ckpt_sha_unchecked fail.
#[test]
fn v34_ckpt_durable_falls_back_to_prev() {
    reset();
    assert_eq!(f::sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", "FIPS 180-4 vector");
    assert_eq!(f::sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    f::set(f::DURABLE, true);
    let dir = std::env::temp_dir().join(format!("sf34-ckpt-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("idx.ckpt");
    let mk = |b: usize| crate::ckpt::Ckpt { fingerprint: "fp".to_owned(), next_b0: b, batches: b, ..crate::ckpt::Ckpt::default() };
    crate::ckpt::save(&p, &mk(1)).unwrap();
    crate::ckpt::save(&p, &mk(2)).unwrap();
    assert_eq!(crate::ckpt::load(&p, "fp", 0, 0).unwrap().unwrap().batches, 2, "the newest generation");
    // an EDITED newest (its body changed, its SHA-256 line kept): fails the line, falls back to the previous generation
    let raw = std::fs::read_to_string(&p).unwrap();
    assert!(raw.contains("POS\t2\t2\t"));
    let good = raw.clone();
    std::fs::write(&p, raw.replace("POS\t2\t2\t", "POS\t2\t7\t")).unwrap();
    assert_eq!(crate::ckpt::load(&p, "fp", 0, 0).unwrap().unwrap().batches, 1, "an edited checkpoint is never read: the previous generation");
    std::fs::write(&p, good).unwrap();
    assert_eq!(crate::ckpt::load(&p, "fp", 0, 0).unwrap().unwrap().batches, 2);
    // tear the newest (a power cut half-way through a write that was not synced)
    let raw = std::fs::read(&p).unwrap();
    std::fs::write(&p, &raw[..raw.len() / 2]).unwrap();
    assert_eq!(crate::ckpt::load(&p, "fp", 0, 0).unwrap().unwrap().batches, 1, "a torn checkpoint falls back to the previous generation");
    std::fs::write(p.with_extension("ckpt.prev"), b"SPIKEFADE-CKPT\t1\tfp\nEND\nSHA256\t00\n").unwrap();
    assert!(crate::ckpt::load(&p, "fp", 0, 0).is_err(), "both torn: refused by name");
    // atomic result file
    let q = dir.join("trades-v3.tsv");
    f::write_atomic(&q, b"a\tb\n").unwrap();
    assert_eq!(std::fs::read(&q).unwrap(), b"a\tb\n");
    assert!(!dir.join("trades-v3.tsv.part").exists());
    for x in ["idx.ckpt", "idx.ckpt.prev", "trades-v3.tsv"] {
        let _ = std::fs::remove_file(dir.join(x));
    }
    let _ = std::fs::remove_dir(&dir);
    reset();
}

/// STANDING F12 (`supervisor_memory_guard_stops_at_checkpoint`): above the gate the run stops at the checkpoint; below it, it goes on; off,
/// never. Mutant m_v34_memguard_never fails.
#[test]
fn v34_memory_guard_stops_above_gate() {
    reset();
    f::MEM_GB.store(1, std::sync::atomic::Ordering::Relaxed);
    assert!(!f::mem_over(2 * 1_048_576), "switch off: never stops");
    f::set(f::MEMGUARD, true);
    assert!(f::mem_over(2 * 1_048_576), "2 GB RSS above a 1 GB gate: stop at this checkpoint");
    assert!(!f::mem_over(512 * 1_024), "512 MB: go on");
    assert!(f::mem_now().0 > 0, "the process RSS is read");
    f::MEM_GB.store(40, std::sync::atomic::Ordering::Relaxed);
    reset();
}

// ------------------------------------------------------------------------------------------------ CARRYSTAT (standing F21)

/// STANDING F21 (`v33o_carry_walk_steps_bounded`): every sale order's carry walk is MEASURED (its blocks read, exact, once per event in the
/// stage A counters) and printed p50 / p99 / max; the walk is bounded by the order window (a slow tape walks every block to 15:10, never
/// more). Mutant m_v34_carry_unmeasured fails.
#[test]
fn v34_carry_walk_measured_and_bounded() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let mut rows = vec![crate::cube::Sec { sod: u32::try_from(t - 1).unwrap(), high: 10_000, low: 10_000, vol: 1_000 }];
    for s in 0..600 {
        rows.push(crate::cube::Sec { sod: u32::try_from(t + s).unwrap(), high: 10_000, low: 10_000, vol: 30 });
    }
    let dt = crate::trade::DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let dl = crate::trade::dial().unwrap();
    let p = crate::trade::record_base(&tape, d, t, 50, 50 * 40, &dl).ok().unwrap();
    assert!(p.walk_blocks > 0, "the sale walk's blocks are read");
    assert!(i64::from(p.walk_blocks) <= (crate::cal::SQ_S - t) / 2 + 1, "never more blocks than the order window holds ({})", p.walk_blocks);
    assert_eq!(f::carry_stats(&[3, 1, 2, 100]), (4, 2, 100, 100), "p50 / p99 / max from exact counts");
    assert_eq!(f::carry_stats(&[]), (0, 0, 0, 0));
    reset();
}

// ------------------------------------------------------------------------------------------------ WICK (matrix X19)

/// MATRIX X19 (`xm_cube_day_high_equals_bhavcopy_high`): a contract whose cube day HIGH (Rs 120.00) is below the NSE bhavcopy HIGH (Rs 135.00)
/// is flagged WICK_DROPPED and listed with both differences; equal extremes give no row; a lower bhavcopy LOW alone flags nothing (only the
/// HIGH side hurts a short). The bound re-prices every unit at the bhavcopy HIGH. Mutant m_v34_wick_low_only fails.
#[test]
fn v34_cube_day_high_vs_bhavcopy_high() {
    reset();
    let d = days_from_civil(2022, 7, 4);
    let mut c = idx_cube(d, vec![(0, 75, vec![75, 75, 75]), (0, 75, vec![75, 75])], &["NIFTY"]);
    for m in 0..3 {
        c.high[Cube::ix(0, m)] = [11_000, 12_000, 10_000][m];
        c.low[Cube::ix(0, m)] = [9_000, 8_000, 9_500][m];
    }
    for m in 0..2 {
        c.high[Cube::ix(1, m)] = [20_000, 21_000][m];
        c.low[Cube::ix(1, m)] = [19_000, 19_500][m];
    }
    let mut bh = std::collections::HashMap::new();
    bh.insert(0, (13_500, 7_500));
    bh.insert(1, (21_000, 18_000));
    let (flagged, checked, rows) = f::wick_compare(&c, &bh);
    assert_eq!(checked, 2);
    assert_eq!(flagged, vec![(0, 13_500)], "only the contract whose cube HIGH is below the bhavcopy HIGH");
    assert_eq!(rows, vec![((d, 1), (12_000, 13_500, 8_000, 7_500))]);
    assert_eq!(f::wick_bound_extra(50, 50 * 12_000, 13_500), 50 * 1_500, "the bound: every unit at the bhavcopy HIGH");
    assert_eq!(f::wick_bound_extra(50, 50 * 14_000, 13_500), 0);
    reset();
}

// ------------------------------------------------------------------------------------------------ DIALS (matrices X07 / X08 / X11 / X12 / X17 / X23)

/// MATRIX X07 (`sf22_lots_capped_by_risk_budget_under_stress`): margin fits 3 lots; a sale at Rs 30.00, lot 350, the stop 5% + the learning-month
/// p99 overshoot 15.61% = 20.61% move, budget Rs 5,000: lots = min(3, floor(5,000 / (0.2061 x 30 x 350))) = 2; budget Rs 10,000: 3; off: 3.
/// Mutant m_v34_riskcap_ignored fails.
#[test]
fn v34_lots_capped_by_risk_budget() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let mut c = idx_cube(d, vec![(0, 350, vec![350])], &["NIFTY"]);
    c.close = vec![3_000; NM];
    let t = 36_000_u32;
    assert_eq!(f::dial_lots(3, &c, 0, t, false, false), 3, "off: the margin fit");
    f::RISK_BUDGET.store(500_000, std::sync::atomic::Ordering::Relaxed);
    f::RISK_OVER_BP.store(2_061, std::sync::atomic::Ordering::Relaxed);
    f::set(f::RISKCAP, true);
    assert_eq!(f::dial_lots(3, &c, 0, t, false, false), 2, "Rs 5,000 budget: 2 lots");
    f::RISK_BUDGET.store(1_000_000, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(f::dial_lots(3, &c, 0, t, false, false), 3, "Rs 10,000 budget: min(3, 4) = 3");
    f::RISK_BUDGET.store(0, std::sync::atomic::Ordering::Relaxed);
    f::RISK_OVER_BP.store(0, std::sync::atomic::Ordering::Relaxed);
    reset();
}

/// MATRIX X08 (`xm_same_side_book_cap`): N = 40, seven CE shorts and one PE: with the cap at 5 per side the 6th and 7th CE are refused
/// (`same_side_cap`, counted by the kernel's codes) and the PE is booked; off: every one booked. Mutant m_v34_sidecap_off fails.
#[test]
fn v34_same_side_book_cap() {
    reset();
    let mut v = Vec::new();
    for k in 0..8_u32 {
        v.push(CapEv { t: 36_000 + 60 * k, margin: 1_000_000, units: 50, sold: 50_000, exit: 50_000, net: 1_000, marks: vec![1_000; NM] });
    }
    let (mut evs, ex, tab) = cap_fixture(&v);
    evs[7].skey |= 0x8000; // the 8th is a PE
    let list: Vec<u32> = (0..8).collect();
    let codes = |evs: &[SEv]| -> Vec<&'static str> { crate::kern::detail3(evs, &ex, &tab, &list, &cs_sl(), 0, 3, None, true, 1).0.iter().map(|d| d.code).collect() };
    assert!(codes(&evs).iter().all(|&c| c == "booked"), "off: every one booked");
    f::SIDE_K.store(5, std::sync::atomic::Ordering::Relaxed);
    f::set(f::SIDECAP, true);
    let c = codes(&evs);
    assert_eq!(&c[..5], &["booked"; 5]);
    assert_eq!((c[5], c[6], c[7]), ("same_side_cap", "same_side_cap", "booked"), "the 6th and 7th CE refused, the PE booked");
    f::SIDE_K.store(u64::MAX, std::sync::atomic::Ordering::Relaxed);
    reset();
}

fn idx_expiry_cube(day: i64, expiry_ymd: u32) -> Cube {
    let mut c = idx_cube(day, vec![(0, 75, vec![75; 300])], &["NIFTY"]);
    c.cons[0].expiry = expiry_ymd;
    c.spot = vec![2_400_000; NM];
    c.close = vec![10_000; NM];
    c
}

/// MATRIX X11 (`xm_expiry_day_split_and_dials`): on an index option's own expiry day, events at 12:30 and 14:00: last sale 12:00 refuses both
/// (`expiry_last_sale`), 13:30 only the 14:00 one, half size halves the lots; a normal day is untouched; off: v3.3. Mutant
/// m_v34_expdial_ignored fails.
#[test]
fn v34_expiry_day_dials() {
    reset();
    let d = days_from_civil(2026, 11, 10);
    let c = idx_expiry_cube(d, ymd_of(d));
    let normal = idx_expiry_cube(d - 1, ymd_of(d));
    let var = std::collections::BTreeMap::new();
    let ev = |c: &Cube, t: u32| crate::trade::size_event_mx(c, 0, t, &var, true, None);
    let (t1230, t1400) = (12 * 3_600 + 30 * 60, 14 * 3_600);
    assert_eq!(ev(&c, t1230).refuse, Refuse::None, "off: v3.3");
    f::EXP_LAST.store(12 * 3_600, std::sync::atomic::Ordering::Relaxed);
    f::set(f::EXPDIAL, true);
    assert_eq!((ev(&c, t1230).refuse, ev(&c, t1400).refuse), (Refuse::ExpiryLastSale, Refuse::ExpiryLastSale), "last sale 12:00");
    f::EXP_LAST.store(13 * 3_600 + 30 * 60, std::sync::atomic::Ordering::Relaxed);
    assert_eq!((ev(&c, t1230).refuse, ev(&c, t1400).refuse), (Refuse::None, Refuse::ExpiryLastSale), "last sale 13:30");
    assert_eq!(ev(&normal, t1400).refuse, Refuse::None, "a normal day is untouched");
    let full = ev(&c, t1230).lots;
    f::EXP_HALF.store(1, std::sync::atomic::Ordering::Relaxed);
    let half = ev(&c, t1230).lots;
    assert!(full.iter().zip(half).all(|(a, b)| b == a / 2) && full.iter().any(|&a| a >= 2), "half size: lots / 2 ({full:?} -> {half:?})");
    f::EXP_LAST.store(0, std::sync::atomic::Ordering::Relaxed);
    f::EXP_HALF.store(0, std::sync::atomic::Ordering::Relaxed);
    reset();
}

/// MATRIX X12 (`xm_event_day_label_and_dial`): a dated event session (2024-06-04, the election results) under `--event-day skip` refuses its
/// sales (`event_day`), `half` halves the lots, `allow` leaves them; the sessions around it are untouched; off: v3.3. Mutant
/// m_v34_eventday_ignored fails.
#[test]
fn v34_event_day_dial() {
    reset();
    let d = days_from_civil(2024, 6, 4);
    let mut cal = f::EventCal::default();
    cal.days.insert(d, ("election results".to_owned(), "ECI".to_owned()));
    f::TEST_EVENTS.with(|x| *x.borrow_mut() = Some(cal));
    let var = std::collections::BTreeMap::new();
    let c = idx_expiry_cube(d, 2024_06_06);
    let before = idx_expiry_cube(d - 1, 2024_06_06);
    let ev = |c: &Cube| crate::trade::size_event_mx(c, 0, 36_000, &var, true, None);
    let base = ev(&c);
    assert_eq!(base.refuse, Refuse::None);
    f::set(f::EVENTCAL, true);
    f::EVENT_MODE.store(2, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(ev(&c).refuse, Refuse::EventDay, "skip");
    assert_eq!(ev(&before).refuse, Refuse::None, "the session before is untouched");
    f::EVENT_MODE.store(1, std::sync::atomic::Ordering::Relaxed);
    assert!(ev(&c).lots.iter().zip(base.lots).all(|(&h, b)| h == b / 2), "half");
    f::EVENT_MODE.store(0, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(ev(&c).lots, base.lots, "allow");
    reset();
}

/// MATRIX X17 (`xm_pick_prints_and_gates_worst_trade_and_p99_loss`): two stock accounts with equal keys, A's worst trade 59.4% of its slot
/// (Rs 4 L at N = 5), B's 3%: the gate at 10% keeps A out of the headline group and B in; off, both. Mutant m_v34_tailgate_ignored fails.
#[test]
fn v34_tail_gate_worst_trade() {
    reset();
    let slot = ACCOUNT / 5;
    let a = Sum { seg: 1, n: 0, worst: -(slot * 594 / 1_000), ..Sum::default() };
    let b = Sum { seg: 1, n: 0, worst: -(slot * 3 / 100), ..Sum::default() };
    assert!(f::tail_ok(&a) && f::tail_ok(&b), "off: both");
    f::TAIL_W.store(10, std::sync::atomic::Ordering::Relaxed);
    f::set(f::TAILGATE, true);
    assert!(!f::tail_ok(&a), "A out: its worst trade lost 59.4% of its slot");
    assert!(f::tail_ok(&b), "B in");
    f::TAIL_W.store(100, std::sync::atomic::Ordering::Relaxed);
    reset();
}

/// MATRIX X23 (`xm_stale_file_flag_and_rescan`): a BOD row with the underlying at Rs 1,000 and a Rs 100 price scan; a decision at Rs 1,040
/// (4% > a third of the scan) is STALE: the strict bound adds |dS| x q = Rs 40 x 50 to the SPAN and to the option value (|delta| <= 1: never
/// below the scan re-priced at the live spot); a fresh decision and the switch off keep the file's parts. Mutant
/// m_v34_strict_span_premium_only fails.
#[test]
fn v34_stale_file_strict_rescan() {
    reset();
    let r = common_fill::margin::Row { und_price: 100_000, price_scan: 10_000, ..common_fill::margin::Row::default() };
    let p = common_fill::margin::Parts { span: 500_000, premium: 200_000, elm: 50_000, exact: 750_000, nse_only: 550_000, rate_h: 350, otm: false };
    assert_eq!(crate::mtable::strict_span(&r, p, 104_000, 50), p, "off: the file's parts");
    f::set(f::STRICTSPAN, true);
    let q = crate::mtable::strict_span(&r, p, 104_000, 50);
    assert_eq!((q.span, q.premium, q.nse_only, q.exact), (700_000, 400_000, 750_000, 1_150_000), "stale: + Rs 40 x 50 on SPAN and on the option value");
    assert_eq!(crate::mtable::strict_span(&r, p, 102_000, 50), p, "a fresh file (2% < a third of the scan): unchanged");
    reset();
}

// ------------------------------------------------------------------------------------------------ READINGS (matrices X14 / X18 / X25 / X26)

/// MATRIX X26 (`sf22_intraday_mtm_drawdown_at_least_realised`): one account, 50 units sold at Rs 100.00 at 10:00, minute-HIGH marks rising to
/// Rs 300.00 at 11:00, bought back at Rs 90.00 at 14:00 (realized +Rs 500; the close-instant drawdown 0): the intraday MTM drawdown is
/// 50 x (300 - 100) = Rs 10,000, the largest running loss the same. Mutant m_v34_mtm_dd_closes_only fails.
#[test]
fn v34_intraday_mtm_drawdown() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let mut marks = vec![10_000; NM];
    marks[105] = 30_000; // 11:00
    for m in marks.iter_mut().skip(106) {
        *m = 9_000;
    }
    let tr = [f::MtmTrade { day: d, t0_us: at_sod(d, 36_000), t1_us: at_sod(d, 50_400), units: 50, sold: 500_000, mark1: 1, net: 50_000 }];
    let (dd, worst) = f::intraday_mtm_dd(&tr, &marks);
    assert_eq!(dd, 1_000_000, "Rs 10,000");
    assert_eq!(worst, 1_000_000);
    // a second day's realized loss alone: the drawdown at least the realized one
    let tr2 = [tr[0], f::MtmTrade { day: d + 1, t0_us: at_sod(d + 1, 36_000), t1_us: at_sod(d + 1, 37_000), units: 50, sold: 500_000, mark1: 1, net: -2_000_000 }];
    assert!(f::intraday_mtm_dd(&tr2, &marks).0 >= 2_000_000);
    reset();
}

/// MATRIX X14 (`sf22_never_closed_next_session_stress`): never-closed units re-priced at the NEXT session's first-minute HIGH of the same
/// contract (a stress beside the net): 50 units valued at Rs 160.00, the next session's first bar HIGH Rs 240.00 -> Rs 4,000 more. Mutant
/// m_v34_x14_no_reprice fails. MATRIX X18: the stress set holds the 2020 crash and halt days, the 2021 outage, 2022-02-24 and 2024-06-04
/// (m_v34_x18_set_empty fails).
#[test]
fn v34_next_session_stress_and_stress_set() {
    reset();
    let d = days_from_civil(2026, 11, 5);
    let mut c = idx_cube(d, vec![(0, 50, vec![0, 50])], &["NIFTY"]);
    c.flags[Cube::ix(0, 0)] = 0;
    c.high[Cube::ix(0, 1)] = 24_000;
    let h = f::first_minute_high(&c, 1).expect("the first bar of the next session");
    assert_eq!(h, 24_000);
    assert_eq!(50 * (h - 16_000), 400_000, "Rs 4,000 more for 50 units");
    assert_eq!(f::first_minute_high(&c, 99), None, "a contract the next session does not list: named, not re-priced");
    let s = f::stress_set();
    for (y, m, dd) in [(2020, 3, 13), (2020, 3, 23), (2021, 2, 24), (2022, 2, 24), (2024, 6, 4)] {
        assert!(s.contains(&days_from_civil(y, m, dd)), "{y}-{m}-{dd} in the stress set");
    }
    reset();
}

/// MATRIX X25 (`xm_respike_ladder_and_first_2_5_10s_printed`): a sale averaging Rs 100.00; the price falls to Rs 70.00 (the best move our way,
/// 30%) before re-spiking to Rs 140.00; the +5% stop fires on a one-second spike that trades back at Rs 102.00 the next second: a STOP HUNT;
/// one that holds above the stop is not. Mutant m_v34_hunt_off fails.
#[test]
fn v34_respike_best_move_and_stop_hunt() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let sec = |s: i64, h: i32, l: i32| crate::cube::Sec { sod: u32::try_from(s).unwrap(), high: h, low: l, vol: 100 };
    let rows = vec![sec(t, 10_000, 10_000), sec(t + 5, 9_000, 7_000), sec(t + 20, 14_000, 13_000), sec(t + 21, 10_200, 10_100), sec(t + 40, 10_600, 10_500), sec(t + 41, 11_000, 10_900)];
    let dt = crate::trade::DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let peak = tape.extreme(at_sod(d, t), at_sod(d, t + 60), true).unwrap();
    assert_eq!(peak, (14_000, at_sod(d, t + 20)));
    assert_eq!(f::best_move_bp(&tape, at_sod(d, t), peak.1, 10_000), 3_000, "30% our way before the re-spike");
    assert!(f::stop_hunt(&tape, at_sod(d, t + 20), 10_500), "the spike reverted the next second: a stop hunt");
    assert!(!f::stop_hunt(&tape, at_sod(d, t + 40), 10_500), "the price held above the stop: no hunt");
    assert!(!f::stop_hunt(&tape, at_sod(d, t + 5), 10_500), "a next second with no print: not a hunt");
    reset();
}

// ------------------------------------------------------------------------------------------------ TTWORDS (exits F-H1)

/// EXITS F-H1 (`v34_exit_why_tt_is_time_armed_breakeven`): a TT exit (the index HEADLINE picks' time-armed breakeven stop) is worded
/// "time-armed breakeven stop" and never "trailing target"; off: v3.3's words. Mutant m_v34_tt_words_trailing fails.
#[test]
fn v34_exit_why_tt_is_time_armed_breakeven() {
    reset();
    assert_eq!(f::tt_word(), "trailing target trigger", "off: v3.3");
    f::set(f::TTWORDS, true);
    assert_eq!(f::tt_word(), "time-armed breakeven stop");
    assert!(!f::tt_word().contains("trailing target"));
    reset();
}

/// STANDING F28 words (build:D-high 14:1x): the RUN.log / RESULTS header names the rule the dated table APPLIES (NSE's regime x sqrt 2 +
/// 0.80% safety, or the first-quarter AEL total as the additional-ELM bound), never Addendum 4's literal rule alone (the plain3 header said
/// only that; the per-trade label was right). Mutant `m_v34_elm_words_literal` prints the old words.
#[test]
fn v34_elm_pre_header_words_name_the_table_rule() {
    let w = f::elm_pre_rule_words();
    assert!(w.contains("x sqrt 2 + 0.80% safety"), "the regime factor and the safety: {w}");
    assert!(w.contains("first-quarter AEL total"), "the additional-ELM bound: {w}");
    assert!(w.contains("not used"), "Addendum 4's literal rule only beside: {w}");
}
