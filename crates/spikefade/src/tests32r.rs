//! v3.2 REPAIR A NAMED TESTS (check-engine-1 M1-M3 / S1 / S3 / S4 / S8 and check-engine-2 M1-M4 / S1-S3 / S5 / S9 of 15 Nov 2026). Every
//! case is synthetic and integer; each test names the defect it guards, and each named mutant (Cargo features m_v32r_*) must make its test
//! FAIL. The switches are per test thread (`sw`); every test starts with every switch off and sets only what it tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_precision_loss, clippy::float_cmp)]

use std::collections::BTreeSet;

use crate::acct::{SEv, Sum};
use crate::cal::{at_sod, days_from_civil, ymd_of};
use crate::cube::{Contract, Cube, Und};
use crate::kern::{CSet, list_keys, walk_account};
use crate::stage::{ABSENT, EvX, NEVER3, NR, Tab3};
use crate::sw;
use crate::trade::{BASE_S, EvT, NL, Out, Refuse};

fn reset() {
    for s in 0..sw::COUNT {
        sw::set(s, false);
    }
}

fn day() -> i64 {
    days_from_civil(2026, 11, 4)
}

fn off(s: i64) -> u16 {
    u16::try_from(s - BASE_S).unwrap()
}

/// The synthetic kernel fixture (as tests3's): per day `per_day` events a minute apart from 10:00, one lot, rule 0 (SL) at T + 10 s (out
/// slot 1, net `net(d, k)`), rule 1 (a TP) at T + 5 s on odd events (out slot 2, half the net), positions lasting `dur(d, k)` seconds.
fn fixture(days: u8, per_day: u32, margin_lot: i64, dur: &dyn Fn(u8, u32) -> u32, net: &dyn Fn(u8, u32) -> i32) -> (Vec<SEv>, Vec<EvX>, Tab3) {
    let mut evs = Vec::new();
    let mut ex = Vec::new();
    let mut tab = Tab3::default();
    for d in 0..days {
        for k in 0..per_day {
            let t = 36_000 + k * 60;
            let et = EvT { refuse: Refuse::None, lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot, filled: [true; NL], ..EvT::default() };
            let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap(), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1: 0 };
            for l in 0..NL {
                let mut a = [ABSENT; NR];
                a[0] = off(i64::from(t) + 10);
                a[1] = if k % 2 == 1 { off(i64::from(t) + 5) } else { NEVER3 };
                let mut o = [0_u8; NR];
                o[0] = 1;
                o[1] = 2;
                tab.anc.push(a);
                tab.os.push(o);
                tab.obase.push(u32::try_from(tab.outs.len()).unwrap());
                let nt = net(d, k) + i32::try_from(l).unwrap();
                let ex0 = u16::try_from(i64::from(t + dur(d, k)) - BASE_S).unwrap();
                tab.outs.push(Out { net: 0, exit: ex0, flag: 0, ..Out::default() });
                tab.outs.push(Out { net: nt, exit: ex0, flag: 1, ..Out::default() });
                tab.outs.push(Out { net: nt / 2, exit: u16::try_from(i64::from(t + dur(d, k) / 2) - BASE_S).unwrap(), flag: 1, ..Out::default() });
            }
            evs.push(SEv { di: u16::from(d), ci: k + 1_000 * u32::from(d), t, skey: k + 1_000 * u32::from(d), tab: et });
            ex.push(x);
        }
    }
    (evs, ex, tab)
}

// ------------------------------------------------------------------------------------------------ check-engine-2 M1 (and S7)

/// check-engine-2 M1: the s26 portfolio counts the running MTM LOSS at every sale (the kernel's rule): member A sold 100 units for Rs 10,000 at
/// 10:00 with Rs 15 L margin and its mark rose to Rs 3,100 (a Rs 3,00,000 loss) by 10:29; member B's sale at 10:30 needs Rs 4 L: the shared
/// cap passes (15 + 4 <= 20 L) and the free money alone passes (5 L >= 4 L), but free money less the loss (2 L) does not: REFUSED and counted
/// as an MTM refusal. Without the open loss (h6) it was taken. The per-minute margin + MTM peak sees the loss.
#[test]
fn v32r_portfolio_cap_counts_mtm() {
    reset();
    let d = day();
    let marks = vec![310_000_i32; crate::cal::NM];
    let a = vec![((at_sod(d, 36_000), at_sod(d, 39_600), 150_000_000_i64, 500_000_i64, d, 1_u64, 11_u64), crate::v32::PMark { units: 100, sold: 1_000_000, mark1: 1 })];
    let b = vec![((at_sod(d, 37_800), at_sod(d, 38_400), 40_000_000_i64, 200_000_i64, d, 2_u64, 12_u64), crate::v32::PMark { units: 50, sold: 500_000, mark1: 0 })];
    let months = crate::v32::months_of(&[d]);
    assert_eq!(crate::v32::loss_at(crate::v32::mark_at(&marks, 1, at_sod(d, 37_800)), 100, 1_000_000), 30_000_000, "the open loss at 10:30");
    let ((net, n, cap, ruled, _, _), mtm) = crate::v32::portfolio_mtm(&[&a, &b], &marks, &months);
    assert_eq!((n, cap, ruled, mtm), (1, 0, 0, 1), "B refused by the running MTM loss (free money Rs 5 L - loss Rs 3 L < Rs 4 L)");
    assert_eq!(net, 500_000);
    // with no open loss B fits
    let flat = vec![10_000_i32; crate::cal::NM];
    let ((_, n2, _, _, _, _), mtm2) = crate::v32::portfolio_mtm(&[&a, &b], &flat, &months);
    assert_eq!((n2, mtm2), (2, 0), "no loss: both taken");
    // the per-minute margin + MTM peak: Rs 15 L margin + Rs 3 L loss = Rs 18 L, never above Rs 20 L
    let taken = vec![(at_sod(d, 36_000), at_sod(d, 39_600), 150_000_000_i64, crate::v32::PMark { units: 100, sold: 1_000_000, mark1: 1 })];
    let (peak, _, _, above, _) = crate::v32::margin_mtm_peak(&taken, &marks, &[d]);
    assert_eq!((peak, above), (180_000_000, 0));
}

// ------------------------------------------------------------------------------------------------ check-engine-2 M2

/// check-engine-2 M2: the which-expiry dial never substitutes. The cube STAGED only the nearest (Mon 9 Nov 2026 weekly) while NSE's previous
/// bhavcopy listed the 17 Nov weekly and the 23 Nov monthly: 'next' and 'monthly' are REFUSED with their label (never the 9 Nov weekly);
/// 'busiest' chooses among the staged candidates and names the missing ones; with the next expiry staged, 'next' reads it; no list held =
/// refused.
#[test]
fn v32r_expiry_dial_never_substitutes() {
    let listed = [2026_11_03_u32, 2026_11_09, 2026_11_17, 2026_11_23, 2026_12_29];
    let today = 2026_11_05_u32;
    let staged = [2026_11_09_u32];
    let nx = crate::expiry::dial_allowed(1, 2026_11_09, Some(&listed), &staged, today);
    assert!(nx.as_ref().is_err_and(|w| w.contains("not staged")), "next: refused, got {nx:?}");
    let mo = crate::expiry::dial_allowed(2, 2026_11_09, Some(&listed), &staged, today);
    assert!(mo.as_ref().is_err_and(|w| w.contains("20261123")), "monthly = the 23 Nov monthly, not staged: refused (never the weekly), got {mo:?}");
    let bz = crate::expiry::dial_allowed(3, 2026_11_09, Some(&listed), &staged, today).unwrap();
    assert_eq!(bz.0, vec![2026_11_09], "busiest among the staged candidates");
    assert_eq!(bz.1.len(), 2, "the next and the monthly candidates named as not staged");
    let staged2 = [2026_11_09_u32, 2026_11_17];
    assert_eq!(crate::expiry::dial_allowed(1, 2026_11_09, Some(&listed), &staged2, today).unwrap().0, vec![2026_11_17]);
    assert!(crate::expiry::dial_allowed(1, 2026_11_09, None, &staged2, today).is_err(), "no list held: refused");
    assert_eq!(crate::expiry::dial_allowed(0, 2026_11_09, None, &staged, today).unwrap().0, vec![2026_11_09], "nearest = v3.1's reading");
}

// ------------------------------------------------------------------------------------------------ check-engine-2 M3

fn special_table() -> crate::expiry::SpecialCal {
    let mut days = std::collections::BTreeMap::new();
    days.insert(days_from_civil(2026, 11, 8), (306_usize, 384_usize, 79_i64, "t".to_owned()));
    days.insert(days_from_civil(2024, 3, 2), (0_usize, 194_usize, 105_i64, "t".to_owned()));
    crate::expiry::SpecialCal { days, path: "t".to_owned(), sha: "s".to_owned() }
}

/// check-engine-2 M3: days to expiry and the theta target's minutes come from ONE dated special-session calendar: the same decision gives the
/// same minutes to expiry and the same sessions with or WITHOUT the special day's cube loaded (h6 added a loaded day's realized span and
/// counted only loaded special days). A loaded special day missing from the table is refused; the 2024 Saturday session counts its 105
/// trading minutes, not its 195-minute span.
#[test]
fn v32r_dte_theta_independent_of_loaded_days() {
    reset();
    let hol = std::env::temp_dir().join(format!("sf32r-hol-{}.tsv", std::process::id()));
    std::fs::write(&hol, "2026-11-10\tDiwali-Balipratipada\n2026-11-08\tDiwali Laxmi Pujan* (Muhurat trading)\n2024-01-26\tRepublic Day\n").unwrap();
    let mut cal = crate::read::Cal3::load(&[hol.as_path()]).unwrap();
    cal.apply_dated(&special_table());
    let (d, e) = (days_from_civil(2026, 11, 5), days_from_civil(2026, 11, 9));
    let before = cal.minutes(d, 40_000, e).unwrap();
    // "load" the Muhurat day: a cube with bars on every slot (a 385-minute realized span unlike the table's 79)
    let mut c = Cube { ymd: ymd_of(days_from_civil(2026, 11, 8)), day: days_from_civil(2026, 11, 8), ..Cube::default() };
    c.flags = vec![5; crate::cal::NM];
    cal.add_specials(std::slice::from_ref(&c));
    let after = cal.minutes(d, 40_000, e).unwrap();
    assert_eq!(before, after, "the theta minutes never depend on the loaded days");
    // 05 Nov 11:06:40 -> 15:30 = 263 min; Fri 6 Nov 375; Sun 8 Nov Muhurat 79; Mon 9 Nov 375
    assert_eq!(before, 263 + 375 + 79 + 375);
    assert!(cal.dated_check(days_from_civil(2026, 11, 8)).is_ok());
    assert!(cal.dated_check(days_from_civil(2026, 11, 7)).is_err(), "a loaded Saturday missing from the table is refused");
    // days to expiry on the dated calendar: Thu 5 Nov -> Mon 9 Nov = Fri, Sun (Muhurat), Mon = 3, with or without the Muhurat cube
    let special: BTreeSet<i64> = cal.special.keys().copied().collect();
    assert_eq!(crate::expiry::sessions_between(d, e, &cal.hol, &special), 3);
    // the 2024 Saturday special session: 105 trading minutes, not its 195-minute span
    let (d2, e2) = (days_from_civil(2024, 3, 1), days_from_civil(2024, 3, 4));
    assert_eq!(cal.minutes(d2, crate::cal::CLOSE_S, e2).unwrap(), 105 + 375);
    let _ = std::fs::remove_file(&hol);
}

// ------------------------------------------------------------------------------------------------ check-engine-2 M4 / S2 / S1 / S9

/// check-engine-2 M4: a walk-forward / train-test row is PASSED only when EVERY test month >= Rs 2,00,000, EVERY test week and EVERY test
/// session are positive (addenda 15 / 19, against the run's sessions); every month passing with one losing week is NOT PASSED; no trade is
/// NOT PASSED.
#[test]
fn v32r_table_w_passed_needs_a19() {
    let mon = days_from_civil(2026, 11, 2);
    let days: Vec<i64> = (0..10).map(|k| mon + k + if k >= 5 { 2 } else { 0 }).collect();
    // two weeks of sessions; every session +Rs 50,000 except one -Rs 10,000 in week 2 (the month passes: Rs 4,40,000)
    let mut tr: Vec<(i64, i64)> = days.iter().map(|&d| (d, 5_000_000)).collect();
    let v = crate::v32::v19_of(&tr, &days);
    assert!(crate::v32::a19_passed(&v, tr.len()), "every month, week and session positive: PASSED");
    tr[7].1 = -1_000_000;
    let v2 = crate::v32::v19_of(&tr, &days);
    assert_eq!(v2.months_ok, (1, 1), "the month still passes");
    assert_eq!(v2.days, (9, 10));
    assert!(!crate::v32::a19_passed(&v2, tr.len()), "one losing session: NOT PASSED");
    // a losing WEEK with every month passing
    let mut tr3: Vec<(i64, i64)> = days.iter().map(|&d| (d, 5_000_000)).collect();
    for x in tr3.iter_mut().skip(5) {
        x.1 = -100;
    }
    tr3[0].1 = 30_000_000;
    let v3 = crate::v32::v19_of(&tr3, &days);
    assert_eq!((v3.months_ok.0, v3.weeks.0), (1, 1), "the month passes, week 2 loses");
    assert!(!crate::v32::a19_passed(&v3, tr3.len()), "a losing week: NOT PASSED");
    assert!(!crate::v32::a19_passed(&crate::v32::v19_of(&[], &days), 0), "no trade: NOT PASSED");
}

/// check-engine-2 S2: under WF19 each row's pick is chosen by the addendum 15 / 19 order on its LEARNING months (more months >= Rs 2 L first),
/// not by v3.1's year rule (every learning year positive, then the smallest DD); the composed pieces carry the month / week / session counts.
#[test]
fn v32r_wf_choice_a19_order() {
    reset();
    let a = crate::wf::Fig { net: 1_000_000_000, mdd: 50_000_000, all_pos: false, mo_ok: 3, wk_pos: 10, d_pos: 40, ..crate::wf::Fig::default() };
    let b = crate::wf::Fig { net: 5_000_000_000, mdd: 10_000_000, all_pos: true, mo_ok: 2, wk_pos: 12, d_pos: 50, ..crate::wf::Fig::default() };
    assert!(crate::wf::learn_key(&b) > crate::wf::learn_key(&a), "v3.1: every year positive wins");
    sw::set(sw::WF19, true);
    assert!(crate::wf::learn_key(&a) > crate::wf::learn_key(&b), "addendum 19: the most months >= Rs 2 L win");
    let p = [crate::kern::Piece { net: 10, mo_ok: 2, wk_pos: 5, d_pos: 20, ..crate::kern::Piece::default() }, crate::kern::Piece { net: 5, mo_ok: 1, wk_pos: 3, d_pos: 9, ..crate::kern::Piece::default() }];
    let f = crate::wf::compose(&p, 0, 1);
    assert_eq!((f.mo_ok, f.wk_pos, f.d_pos), (3, 8, 29));
    reset();
}

/// check-engine-2 S2 (kernel): the per-piece counters of a walked account under WF19: three sessions (piece 0: +Rs 1,50,000 Mon, +Rs 50,000
/// Tue of the next week; piece 1: +Rs 2,50,000 in the next month) give piece 0 = 1 month >= Rs 2 L, 2 positive weeks, 2 positive sessions;
/// piece 1 = 1 / 1 / 1.
#[test]
fn v32r_kernel_piece_counters() {
    reset();
    sw::set(sw::WF19, true);
    let m: &'static [u16] = Box::leak(vec![0_u16, 0, 1].into_boxed_slice());
    let w: &'static [u16] = Box::leak(vec![0_u16, 1, 2].into_boxed_slice());
    crate::kern::TEST_MAPS.with(|c| c.set(Some((m, w))));
    let nets = [15_000_000_i32, 5_000_000, 25_000_000];
    let (evs, ex, tab) = fixture(3, 1, 1_000_000, &|_, _| 60, &|d, _| nets[usize::from(d)] - i32::try_from(0).unwrap());
    let list: Vec<u32> = (0..3).collect();
    let keys = list_keys(&evs, &list);
    let of_day: Vec<u8> = vec![0, 0, 1];
    let cs = CSet { rules: vec![(false, 0)], sl: 0 };
    let (_, pc, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 0, None, true, false, &of_day, crate::wf::NPIECE, 3);
    assert_eq!((pc.p[0].mo_ok, pc.p[0].wk_pos, pc.p[0].d_pos), (1, 2, 2), "piece 0");
    assert_eq!((pc.p[1].mo_ok, pc.p[1].wk_pos, pc.p[1].d_pos), (1, 1, 1), "piece 1");
    crate::kern::TEST_MAPS.with(|c| c.set(None));
    reset();
}

/// check-engine-2 S1: weeks counted against the RUN's weeks: trades only in week 1 of a two-week run give 1 positive week of 2 and the worst
/// week Rs 0 (the week without a trade is net 0: not positive), never "1 of 1 seen".
#[test]
fn v32r_weeks_against_run_weeks() {
    let mon = days_from_civil(2026, 11, 2);
    let days: Vec<i64> = vec![mon, mon + 1, mon + 7, mon + 8];
    let v = crate::v32::v19_of(&[(mon, 5_000_000)], &days);
    assert_eq!(v.weeks, (1, 2));
    assert_eq!(v.worst_week, (0, mon + 7), "the unseen week, by its Monday");
    assert_eq!(v.days, (1, 4));
    assert!(crate::v32::v19_miss_words(&v).contains("week of 2026-11-09"), "the week gap named: {}", crate::v32::v19_miss_words(&v));
}

/// check-engine-2 S9 (addendum 19): the verdict view orders months >= Rs 2 L, then positive weeks, then positive SESSIONS, then the smallest
/// max DD; the worst day carries its date.
#[test]
fn v32r_a19_order_and_worst_dates() {
    reset();
    let a = Sum { trades: 9, mo_ok: 1, wk_pos: 2, days_pos: 5, mdd: 9_000_000, net: 1_000, ..Sum::default() };
    let b = Sum { trades: 9, mo_ok: 1, wk_pos: 2, days_pos: 3, mdd: 1_000_000, net: 1_000, ..Sum::default() };
    assert!(crate::acct::verdict_key(&b) > crate::acct::verdict_key(&a), "h6: sessions not in the order (the smaller DD wins)");
    sw::set(sw::A19, true);
    assert!(crate::acct::verdict_key(&a) > crate::acct::verdict_key(&b), "addendum 19: more positive sessions first");
    let mon = days_from_civil(2026, 11, 2);
    let v = crate::v32::v19_of(&[(mon, 100), (mon + 1, -700), (mon + 2, 50)], &[mon, mon + 1, mon + 2]);
    assert_eq!(v.worst_day, (-700, mon + 1));
    assert!(crate::v32::v19_words(&v).contains("worst day Rs -7.00 on 2026-11-03"), "{}", crate::v32::v19_words(&v));
    reset();
}

// ------------------------------------------------------------------------------------------------ check-engine-2 S3

fn cube(kind: u8, name: &str, expiry: u32, lot: u32) -> Cube {
    let d = day();
    let mut c = Cube { ymd: ymd_of(d), day: d, ..Cube::default() };
    c.unds.push(Und { name: name.to_owned(), kind, spot_seg: kind, spot_sid: 1, nearest: expiry, next: 0 });
    c.cons.push(Contract { sid: 7, und: 0, kind, right: 0, expiry, lot, strike: 100_000, first_min: 0, first_open: 1_000, symbol: format!("{name}-X-CE"), n_sec: 0 });
    c.spot = vec![100_000; crate::cal::NM];
    c.close = vec![1_000; crate::cal::NM];
    c.flags = vec![5; crate::cal::NM];
    c.high = vec![1_000; crate::cal::NM];
    c.low = vec![1_000; crate::cal::NM];
    c.vol = vec![500; crate::cal::NM];
    c.oi = vec![0; crate::cal::NM];
    c
}

/// check-engine-2 S3: a per-segment slot-count grid inside 5..100: the index grid 5 / 10 / 15 / 20 (an N = 40 index slot of Rs 50,000 could
/// never hold a lot) and the stock grid 5 / 10 / 20 / 40; the sizing and the kernel's capacity read the segment's grid.
#[test]
fn v32r_ngrid_per_segment() {
    reset();
    assert_eq!((sw::n_slots(0, 3), sw::n_slots(1, 3)), (40, 40), "off: v3.1's grid");
    sw::set(sw::NGRID, true);
    assert_eq!((0..4).map(|i| sw::n_slots(0, i)).collect::<Vec<_>>(), vec![5, 10, 15, 20], "index grid");
    assert_eq!((0..4).map(|i| sw::n_slots(1, i)).collect::<Vec<_>>(), vec![5, 10, 20, 40], "stock grid");
    assert_eq!(crate::kern::cap_at(0, 2), 15);
    let mut var = std::collections::BTreeMap::new();
    var.insert("NIFTY".to_owned(), crate::margin::VarRow { var_x100: 1_200, elm_x100: 350 });
    let c = cube(0, "NIFTY", ymd_of(day() + 5), 75);
    let e = crate::trade::size_event_mx(&c, 0, 36_000, &var, true, None);
    assert_eq!(e.seg, 0);
    if e.margin_lot > 0 {
        assert_eq!(i64::from(e.lots[2]), (crate::trade::ACCOUNT / 15) / e.margin_lot, "slot index 2 = N 15 for an index option");
    }
    reset();
}

// ------------------------------------------------------------------------------------------------ check-engine-1 M1

/// check-engine-1 M1: under CHG_DATED the charges are at the session's DATED rates only (2023-03: STT 0.05% on the sold premium), never the
/// dearer of the dated and the 2026-10-30 sets (STT 0.15%); the dearer-of reading is the one printed beside.
#[test]
fn v32r_charges_dated_only() {
    reset();
    let d = days_from_civil(2023, 3, 15);
    let (units, buy, sell) = (1_500_u64, 2_000_000_i64, 6_000_000_i64);
    let dearer = crate::trade::charges_orders_n(d, units, buy, sell, 1, 1, 0).unwrap();
    assert!(dearer.3, "h6: the 2026 set is the dearer for 2023-03");
    sw::set(sw::CHG_DATED, true);
    let dated = crate::trade::charges_orders_n(d, units, buy, sell, 1, 1, 0).unwrap();
    let rd = crate::charges::rates_on(d).unwrap();
    let (b, s) = crate::charges::charges_est_sell(&rd, &crate::charges::Trip { qty: 1_500, buy_value: buy, sell_value: sell, entry_orders: 1, exit_orders: 1, ..crate::charges::Trip::default() });
    assert_eq!((dated.0, dated.1, dated.2, dated.3), (b + s, b, s, false), "the dated set only");
    assert!(dated.0 < dearer.0, "STT 0.05% (dated) below 0.15% (2026)");
    assert_eq!(crate::trade::charges_dearer_of(d, units, buy, sell, (1, 1, 0)), dearer.0, "the dearer-of reading beside");
    reset();
}

// ------------------------------------------------------------------------------------------------ check-engine-1 M2

/// check-engine-1 M2: addendum 14 (b) makes liquidity a HARD gate: under HL_STRICT no account whose list has the liquidity gate OFF (value 0)
/// enters the HEADLINE group; a gated list (C30G10) does.
#[test]
fn v32r_headline_excludes_liquidity_off() {
    reset();
    sw::set(sw::HEADLINE, true);
    let sm = Sum { trades: 5, net_s2: 1_000, ..Sum::default() };
    assert!(crate::report3::headline_gate_liq(true, 0, 0, &sm), "h6: liquidity OFF admitted");
    sw::set(sw::HL_STRICT, true);
    assert!(!crate::report3::headline_gate_liq(true, 0, 0, &sm), "liquidity OFF: never a headline");
    assert!(crate::report3::headline_gate_liq(true, 0, 7, &sm), "C30G10: admitted");
    reset();
}

// ------------------------------------------------------------------------------------------------ check-engine-1 M3 / S4

/// check-engine-1 M3: every rule-fired row says HOW the level was set in rupees ("stop = R(s) Rs 18.90 + Rs 0.95 ... = Rs 19.85"), and a HELD
/// row (fired inside 15:08-15:10) names the rule, the TRIGGER second (not the anchor) and the level.
#[test]
fn v32r_exit_why_held_rule_level_trigger() {
    let xr = common_fill::xrule::XRule::Stop { x: 95 };
    let how = crate::v32::level_words(Some(&xr), 1_985, 1_890, "SL 5% of the premium above R(s)");
    assert!(how.contains("stop = R(s) Rs 18.90 + Rs 0.95"), "{how}");
    assert!(how.contains("= Rs 19.85"), "{how}");
    let held = crate::v32::held_why("stop-loss", "SL 5% of the premium above R(s)", "SL.P.5=Stop { x: 1305 }", "15:08:30", "stop = R(s) Rs 248.00 + Rs 13.05 = Rs 261.05", 26_105, "15:08:31", "our own 15:10 square-off bought 65 units", "0 units unfilled");
    assert!(held.starts_with("stop-loss (SL 5%"), "the rule word: {held}");
    assert!(held.contains("fired at 15:08:30"), "the trigger second, not the anchor: {held}");
    assert!(held.contains("Rs 261.05"), "the level in rupees: {held}");
    let tp = crate::v32::level_words(Some(&common_fill::xrule::XRule::Target { x: 300 }), 1_590, 1_890, "TP 15%");
    assert!(tp.contains("target = R(s) Rs 18.90 - Rs 3.00"), "{tp}");
}

/// check-engine-1 S4: percentages rounded HALF UP to 2 decimals, never truncated: 2 of 3 units = 66.67%; Rs 400.00 -> Rs 1,212.03 = +203.01%.
#[test]
fn v32r_percent_round_half_up() {
    assert_eq!(crate::v32::bp_round(2, 3), 6_667);
    assert_eq!(crate::v32::bp_round(81_203, 40_000), 20_301);
    assert_eq!(crate::v32::bp_round(-81_203, 40_000), -20_301, "half away from zero below 0");
    assert!(crate::v32::gain_words_r(40_000, 121_203, "x").starts_with("+203.01%"), "{}", crate::v32::gain_words_r(40_000, 121_203, "x"));
    assert!(crate::v32::gain_words(40_000, 121_203, "x").starts_with("+203.00%"), "h6 truncated");
}

// ------------------------------------------------------------------------------------------------ speed-up 4 wired

/// SPEED-UP 4 WIRED (exact setting dedup in stage B): the dedup walk gives every setting the SAME summary and pieces as the plain chunk
/// kernel, walking fewer accounts (settings 0 and 2 decide alike on every event).
#[test]
fn v32r_stage_b_dedup_equals_plain() {
    reset();
    let (evs, ex, tab) = fixture(2, 30, 1_000_000, &|_, k| 60 + (k % 5) * 90, &|_, k| i32::try_from(k % 9).unwrap() * 3_000 - 12_000);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let sets = vec![CSet { rules: vec![(false, 0)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 1)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 2)], sl: 0 }];
    let of_day: Vec<u8> = vec![0, 1];
    let (plain, s1) = crate::kern::walk_chunk(&evs, &ex, &tab, &list, &keys, &sets, None, true, true, &of_day, crate::wf::NPIECE, 2);
    let (dd, s2, walked) = crate::kern::walk_dedup(&evs, &ex, &tab, &list, &keys, &sets, None, true, true, &of_day, crate::wf::NPIECE, 2, None);
    assert_eq!(plain.len(), dd.len());
    for (a, (x, y)) in plain.iter().zip(&dd).enumerate() {
        assert!(crate::report3::sum_eq_all(&x.0, &y.0), "account {a}: summaries differ");
        assert_eq!(x.1.p, y.1.p, "account {a}: pieces differ");
    }
    assert_eq!(walked, 2 * 5 * 4, "settings 0 and 2 walk one account per (share, N)");
    assert!(s2 < s1, "fewer steps walked");
}

// ------------------------------------------------------------------------------------------------ check-engine-2 S5: O(1) gates

fn p99_best_of(trials: usize, f: &mut dyn FnMut() -> Vec<u64>) -> u64 {
    let mut best = u64::MAX;
    for _ in 0..trials {
        let mut v = f();
        v.sort_unstable();
        best = best.min(v.get(v.len().saturating_sub(1) * 99 / 100).copied().unwrap_or(0));
    }
    best
}

/// check-engine-2 S5 (matrix X65 / X68): the range max is O(1) at p99 (the crate's RangeMm through `Tape::extreme`): a 6-hour range costs no
/// more than 10x a 10-second one and each stays under 20 us at p99 (best of 7 trials, wall clock, a loaded machine); `first_reach` over a
/// whole session takes at most 16 binary-lifting steps.
#[test]
fn v32r_o1_range_max_and_first_reach() {
    let d = day();
    let rows: Vec<crate::cube::Sec> = (0..22_500_i64).map(|k| crate::cube::Sec { sod: u32::try_from(crate::cal::OPEN_S + k).unwrap(), high: i32::try_from(1_000 + (k * 7_919) % 5_000).unwrap(), low: i32::try_from(900 + (k * 7_919) % 5_000).unwrap(), vol: 50 }).collect();
    let dt = crate::trade::DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let t0 = at_sod(d, crate::cal::OPEN_S);
    let mut q = |len: i64| -> Vec<u64> {
        (0..2_000_i64).map(|k| {
            let a = t0 + ((k * 7_907) % (22_000 - len.min(21_000))) * crate::cal::US_S;
            let s = std::time::Instant::now();
            let r = tape.extreme(a, a + len * crate::cal::US_S, true);
            let ns = u64::try_from(s.elapsed().as_nanos()).unwrap();
            assert!(r.is_some());
            ns
        }).collect()
    };
    let short = p99_best_of(7, &mut || q(10));
    let long = p99_best_of(7, &mut || q(21_000));
    assert!(short < 20_000 && long < 20_000, "range max p99 short {short} ns, long {long} ns");
    assert!(long <= 10 * short.max(200), "O(1): a 6-hour range at most 10x a 10-second one (short {short}, long {long})");
    let mut steps = 0_u32;
    let _ = tape.first_reach(t0, t0 + 22_400 * crate::cal::US_S, true, 1_000_000, &mut steps);
    assert!(steps <= 16, "first_reach over a session: {steps} steps (gate 16)");
}

/// check-engine-2 S5 (matrix X67, the stage B step): a decision with N = 100 open positions stays O(N) and fast: 3,000 events booked into a
/// 100-slot account (positions lasting all day) step at p99 under 50 us (best of 5, wall clock), and the sampled step blocks of the stage B
/// kernel are recorded and give a step-weighted p99.
#[test]
fn v32r_o1_decision_at_n100_and_sampled_steps() {
    reset();
    sw::set(sw::NGRID, true);
    sw::NGRID_T.with(|g| g.set(Some([[5, 10, 20, 100], [5, 10, 20, 40]])));
    let (evs, ex, tab) = fixture(1, 300, 100_000, &|_, _| 20_000, &|_, k| i32::try_from(k % 7).unwrap() * 100 - 300);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let cs = CSet { rules: vec![(false, 0)], sl: 0 };
    let p99 = p99_best_of(5, &mut || {
        let mut ac = crate::kern::Acc::new();
        let mut p = vec![crate::kern::Pos::default(); 100];
        let mut pc = crate::kern::Pieces::default();
        list.iter().map(|&e| {
            let s = std::time::Instant::now();
            let _ = crate::kern::step3(&mut ac, &mut p, &evs[e as usize], &ex[e as usize], &tab, &cs, 0, 3, None, true, &mut pc, 0);
            u64::try_from(s.elapsed().as_nanos()).unwrap()
        }).collect()
    });
    assert!(p99 < 50_000, "a decision at N = 100: p99 {p99} ns");
    let (rows, _) = crate::kern::walk_chunk(&evs, &ex, &tab, &list, &keys, std::slice::from_ref(&cs), None, true, true, &[], 0, 1);
    assert_eq!(rows[3].0.trades, 100, "the 100-slot account fills its 100 slots");
    let mut blocks = Vec::new();
    let _ = crate::kern::walk_chunk_s(&evs, &ex, &tab, &list, &keys, std::slice::from_ref(&cs), None, true, false, &[], 0, 1, Some(&mut blocks));
    assert!(blocks.len() >= 4, "every 64th event's steps sampled: {} blocks", blocks.len());
    let (p50, p99s, n) = crate::kern::step_weighted(&blocks);
    assert!(n == blocks.len() && p50 <= p99s && p99s > 0);
    sw::NGRID_T.with(|g| g.set(None));
    reset();
}

/// check-engine-2 S5 (matrix X66): the margin lookup is a flat index read: 20,000 contracts, every lookup under 20 us at p99 (best of 5).
#[test]
fn v32r_o1_margin_lookup_ns() {
    reset();
    let d = day();
    let mut t = String::from("date\tund\tseg\texpiry\tright\tstrike\tfile_price\tund_file_price\tspan_unit_short\tscan_unit\tworst_scenario\tsom_unit\telm_rate_pct\telm_class\telm_oth_pct\telm_otm_pct\tlot\tdelta\tseries_price_scan\tfile\tfile_created\tael_file\n");
    for k in 0..20_000 {
        t.push_str(&format!("2026-11-04\tSBIN\tstk\t20261111\tC\t{}\t10\t900\t100\t100\t11\t0\t5.25\tOTM\t3.5\t5.25\t\t0.5\t142\tnsccl.20261104.i1\tx\tael\n", 1_000 + k));
    }
    let s = common_fill::margin::Snapshot::parse(t.as_bytes()).unwrap();
    let dm = crate::mtable::DayM { day: d, fallback_from: None, snaps: vec![(i64::MIN, "nsccl.20261104.i1".to_owned(), s)] };
    let mut c = cube(1, "SBIN", 2026_11_11, 50);
    for k in 1..2_000_i64 {
        let mut x = c.cons[0].clone();
        x.strike = (1_000 + k * 7) * 100;
        c.cons.push(x);
    }
    let mut lat = crate::mtable::Lat::default();
    let cm = crate::mtable::resolve(&dm, &c, &mut lat);
    sw::set(sw::MARGIN_V2, true);
    let p99 = p99_best_of(5, &mut || {
        (0..2_000_usize).map(|ci| {
            let s = std::time::Instant::now();
            let _ = crate::mtable::exact_at(&dm, &cm, ci, at_sod(d, 36_000), 90_000, 50, ymd_of(d), None);
            u64::try_from(s.elapsed().as_nanos()).unwrap()
        }).collect()
    });
    assert!(p99 < 20_000, "margin lookup p99 {p99} ns");
    reset();
}

/// v3.3 integrate (tests33f): the kernel fixture above with 600-s positions and a fixed net, for the day-major step-sampling test.
pub fn fixture_pub(days: u8, per_day: u32, margin_lot: i64) -> (Vec<SEv>, Vec<EvX>, Tab3) {
    fixture(days, per_day, margin_lot, &|_, _| 600, &|_, k| i32::try_from(k % 5).unwrap() * 100 - 100)
}
