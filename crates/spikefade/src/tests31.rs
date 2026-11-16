//! v3.1 NAMED TESTS (MAIN-RULINGS-ENGINE-v22 items 1-8 and addenda 1-3; the operator's decisions (a)-(d); check-build-v3 D1-D3 and S1; the
//! extreme-case matrix of addendum 2 and EXTREME-MATRIX-SELLING.md). Every case is synthetic and integer; each test names the rule it
//! guards, and the named mutants (Cargo features m_v31_*) must make their test FAIL. The switches are per test thread here (`sw`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]

use common_fill::margin::{Row, Snapshot};

use crate::acct::{SEv, Sum, Tops};
use crate::cal::{CLOSE_S, OPEN_S, SQ_S, at_sod, days_from_civil, ymd_of};
use crate::cube::{Contract, Cube, Sec, Und};
use crate::kern::{CSet, detail3, list_keys, walk_account};
use crate::mtable::{ConM, DayM, Manifest, MarginAt, SnapMeta, exact_at};
use crate::stage::{ABSENT, EvX, NEVER3, NR, Tab3};
use crate::sw;
use crate::trade::{ACCOUNT, BASE_S, DayTape, EvT, NL, Out, Refuse, dial, never_closed_value, record_base, size_event_mx, trip};

fn day() -> i64 {
    days_from_civil(2026, 11, 4)
}

fn sec(s: i64, h: i32, l: i32, v: u32) -> Sec {
    Sec { sod: u32::try_from(s).unwrap_or(0), high: h, low: l, vol: v }
}

fn off(s: i64) -> u16 {
    u16::try_from(s - BASE_S).unwrap_or(0)
}

/// Every switch back to v3 (each test starts from v3 and sets only what it tests).
fn reset() {
    for s in 0..12 {
        sw::set(s, false);
    }
}

// ---------------------------------------------------------------- item 3: orders end at 15:30; never-closed units

/// ITEM 3a (F-B): with the switch, the square-off walks to 15:30:00 only: a tape whose only prints after 15:10 are at 15:35 leaves the units
/// NEVER CLOSED (valued, not filled), known at 15:30; v2.1 (the switch off) fills them at 15:35.
#[test]
fn v31_orders_end_1530_no_fill_after() {
    reset();
    let d = day();
    let t = 36_000_i64;
    // after 15:10 the only prints are at 15:35:00-15:35:03 (the first after a long silence: the FA1 prorate reading counts little of it;
    // the next ones are whole seconds after a print: their volume counts)
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000), sec(t, 10_000, 9_900, 1_000), sec(t + 1, 10_000, 9_900, 1_000)];
    for k in 0..4 {
        rows.push(sec(15 * 3_600 + 35 * 60 + k, 12_000, 11_000, 5_000));
    }
    let dl = dial().unwrap();
    for on in [false, true] {
        sw::set(sw::END_1530, on);
        sw::set(sw::VALUE_FROM_EXIT, on);
        let dt = DayTape::new(d, &rows, 2).unwrap();
        let tape = dt.tape().unwrap();
        let mut p = record_base(&tape, d, t, 50, 50, &dl).ok().unwrap();
        let r = trip(&mut p, &tape, d, 50, 4, 50, None, &dl).unwrap();
        assert_eq!(r.units, 50);
        if on {
            assert_eq!((r.sq_units, r.unclosed), (0, 50), "no fill after 15:30");
            assert_eq!(r.known, at_sod(d, CLOSE_S), "a never-closed position is known at 15:30");
            assert!(r.val_fallback, "no print between the 15:10 decision and 15:30: valued over the entry window, counted");
        } else {
            assert_eq!((r.sq_units, r.unclosed), (50, 0), "v2.1: filled at the 15:35 print");
        }
    }
    reset();
}

/// ITEM 3b: never-closed units are valued at the highest HIGH between the EXIT DECISION and 15:30 (v2.1: from the entry E0 to the session
/// end, which held a spike no buy-back could have paid and a 15:35 print after the close).
#[test]
fn v31_never_closed_valued_from_exit_decision() {
    reset();
    let d = day();
    let t = 36_000_i64;
    // entry at 10:00 (Rs 100), a spike to Rs 300 at 10:05, Rs 150 thin prints afterwards, Rs 160 at 15:20, Rs 400 at 15:35
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000), sec(t, 10_000, 10_000, 1_000), sec(t + 300, 30_000, 29_000, 1)];
    for k in 0..300 {
        rows.push(sec(t + 600 + k * 60, 15_000, 14_900, 1));
    }
    rows.push(sec(15 * 3_600 + 20 * 60, 16_000, 15_900, 1));
    rows.push(sec(15 * 3_600 + 35 * 60, 40_000, 39_000, 1));
    rows.sort_by_key(|r| r.sod);
    rows.dedup_by_key(|r| r.sod);
    let dt = DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let e0 = at_sod(d, t);
    let xd = at_sod(d, 12 * 3_600);
    // v2.1: the highest HIGH from E0 to 15:40: the 15:35 print, Rs 400
    assert_eq!(never_closed_value(&tape, d, e0, xd).0, 40_000);
    sw::set(sw::END_1530, true);
    sw::set(sw::VALUE_FROM_EXIT, true);
    let (v, end, fb) = never_closed_value(&tape, d, e0, xd);
    assert_eq!((v, end, fb), (16_000, at_sod(d, CLOSE_S), false), "from the 12:00 exit decision to 15:30: Rs 160");
    // the decision of a square-off is 15:10: the same window here
    assert_eq!(never_closed_value(&tape, d, e0, at_sod(d, SQ_S)).0, 16_000);
    reset();
}

// ---------------------------------------------------------------- items 4 and 5: the MTM check; grouped drawdown

/// A kernel fixture: events at the given seconds on distinct contracts, one lot each (margin `m` per lot), each closing at its own exit
/// second with net `net`, sold `units` units for `sold` paisa; the contract-day marks of event i are `marks(i, minute)`.
fn fixture(ev: &[(u32, i64, u32, i32, u32, i64)], marks: &dyn Fn(usize, usize) -> i32) -> (Vec<SEv>, Vec<EvX>, Tab3) {
    let mut evs = Vec::new();
    let mut ex = Vec::new();
    let mut tab = Tab3::default();
    for (i, &(t, m, exit, net, units, sold)) in ev.iter().enumerate() {
        let et = EvT { refuse: Refuse::None, lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot: m, filled: [true; NL], ..EvT::default() };
        let m0 = u32::try_from(tab.marks.len()).unwrap();
        tab.marks.extend((0..crate::cal::NM).map(|k| marks(i, k)));
        let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap(), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1: m0 + 1 };
        for _ in 0..NL {
            let mut a = [ABSENT; NR];
            a[0] = NEVER3;
            tab.anc.push(a);
            tab.os.push([0; NR]);
            tab.obase.push(u32::try_from(tab.outs.len()).unwrap());
            tab.outs.push(Out { net, exit: off(i64::from(exit)), flag: 1, units, sold, ..Out::default() });
        }
        let ci = u32::try_from(i).unwrap();
        evs.push(SEv { di: 0, ci, t, skey: ci, tab: et });
        ex.push(x);
    }
    (evs, ex, tab)
}

/// ITEM 4 (F-C): available = Rs 20 L + min(0, realized) - margin in use - the open positions' unrealised loss. A second position that fits the
/// plain cash rule is REFUSED (margin_mtm_short, counted) when the first one's running loss eats the money; the switch off books it.
#[test]
fn v31_mtm_check_refuses_with_running_loss() {
    reset();
    let m = 39_000_000_i64; // Rs 3.9 L a lot (N 5: slot Rs 4 L)
    // A at 10:00 sold 100 units for Rs 1,000 (Rs 10 each), open to 15:10; its option trades at Rs 17,000 from 10:30 (an absurd loss on purpose)
    let ev = [(36_000_u32, m, 15 * 3_600 + 10 * 60, -1_000, 100_u32, 100_000_i64), (39_600, m, 15 * 3_600 + 10 * 60, 500, 100, 100_000)];
    let marks = |i: usize, k: usize| if i == 0 && k >= 75 { 1_700_000 } else { 1_000 };
    let (evs, ex, tab) = fixture(&ev, &marks);
    let list: Vec<u32> = vec![0, 1];
    let cs = CSet { rules: vec![(false, 0)], sl: 0 };
    let (d0, s0, _) = detail3(&evs, &ex, &tab, &list, &cs, 0, 0, None, true, 1);
    assert_eq!(d0.iter().map(|x| x.code).collect::<Vec<_>>(), vec!["booked", "booked"], "the switch off: v2.1's cash rule books both");
    assert_eq!(s0.mtm_ref, 0);
    sw::set(sw::MTM, true);
    let (d1, s1, _) = detail3(&evs, &ex, &tab, &list, &cs, 0, 0, None, true, 1);
    // unrealised loss of A at 11:00 = 100 x Rs 17,000 - Rs 1,000 = Rs 16,99,000: free 20 L - 3.9 L - 16.99 L < 3.9 L
    assert_eq!(d1.iter().map(|x| x.code).collect::<Vec<_>>(), vec!["booked", "margin_mtm_short"]);
    assert_eq!(s1.mtm_ref, 1, "the MTM refusal is counted");
    assert_eq!(crate::kern::unrealised(&[crate::kern::Pos { units: 100, sold: 100_000, mark1: ex[0].mark1, ..crate::kern::Pos::default() }], &tab, 39_600), 100 * 1_700_000 - 100_000);
    reset();
}

/// ITEM 5 (v2.1 S-3, 2026 F5): max DD on the equity at each close INSTANT: two positions closing in the same second (-Rs 1 L then +Rs 1 L in
/// booking order) make no drawdown when grouped; the booking-order figure is printed beside.
#[test]
fn v31_dd_grouped_same_second_closes() {
    reset();
    let x = 11 * 3_600;
    let ev = [(36_000_u32, 1_000_000_i64, x, -10_000_000_i32, 1_u32, 1_i64), (36_060, 1_000_000, x, 10_000_000, 1, 1), (36_120, 1_000_000, x + 600, 5_000_000, 1, 1)];
    let (evs, ex, tab) = fixture(&ev, &|_, _| 0);
    let list: Vec<u32> = vec![0, 1, 2];
    let keys = list_keys(&evs, &list);
    let cs = CSet { rules: vec![(false, 0)], sl: 0 };
    let (a, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 0, None, true, false, &[], 0, 1);
    assert_eq!((a.mdd, a.mdd_alt), (10_000_000, 0), "v2.1 booking order: Rs 1 L drawdown; the grouped figure beside is 0");
    sw::set(sw::DD_GROUPED, true);
    let (b, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 0, None, true, false, &[], 0, 1);
    assert_eq!((b.mdd, b.mdd_alt), (0, 10_000_000), "grouped: the same-second closes net to 0; the booking-order figure beside");
    assert_eq!(a.net, b.net);
    reset();
}

/// ITEM 3 (2026 F1): never-closed trades, units and their net are carried to every account row (valued, not filled), never hidden.
#[test]
fn v31_never_closed_counted_per_account() {
    reset();
    let mut ev = [(36_000_u32, 1_000_000_i64, 13 * 3_600, -50_000_i32, 75_u32, 1_000_i64), (36_600, 1_000_000, 13 * 3_600 + 60, 20_000, 75, 1_000)];
    ev[0].4 = 75;
    let (evs, ex, mut tab) = fixture(&ev, &|_, _| 0);
    // the first trade's outs carry 75 never-closed units
    for o in tab.outs.iter_mut().take(NL) {
        o.nc_units = 75;
    }
    let list: Vec<u32> = vec![0, 1];
    let keys = list_keys(&evs, &list);
    let cs = CSet { rules: vec![(false, 0)], sl: 0 };
    let (a, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 0, None, true, false, &[], 0, 1);
    assert_eq!((a.nc_trades, a.nc_units, a.nc_net), (1, 75, -50_000));
    reset();
}

// ---------------------------------------------------------------- decisions (a)-(c): the headline gate; the S1 tie rule

/// DECISIONS (a)-(c): the HEADLINE group admits only a SEARCH (small-stop) setting at share 10% or 25% that stays positive at +2% adverse on
/// both sides.
#[test]
fn v31_headline_gate_small_stop_share_25_stress_positive() {
    let ok = Sum { net_s2: 1, ..Sum::default() };
    let neg = Sum { net_s2: 0, ..Sum::default() };
    assert!(crate::report3::headline_gate(true, 0, &ok) && crate::report3::headline_gate(true, 1, &ok));
    assert!(!crate::report3::headline_gate(true, 2, &ok), "50% share: a labelled comparison, never the headline");
    assert!(!crate::report3::headline_gate(true, 4, &ok), "100% share: a labelled comparison");
    assert!(!crate::report3::headline_gate(false, 0, &ok), "a wide (COMPARISON) stop is never the headline");
    assert!(!crate::report3::headline_gate(true, 0, &neg), "not positive at +2% adverse both sides");
}

/// check-build-v3 S1 (design 5.2): among accounts of equal ranking key, fewer families on first, then the SL kind rank and the smaller level,
/// then the setting's file order, then list, share and N. The v3 order (list first) is kept with the switch off (P0).
#[test]
fn v31_tie_rule_design52() {
    reset();
    let base = Sum { trades: 3, net: 1_000, ndays: 1, days_pos: 1, mdd: 10, ..Sum::default() };
    // a: list 1, setting slot 7, 3 families; b: list 2, setting slot 3, 2 families
    let a = Sum { list: 1, x: 7 * 5, tie: (3 << 24) | 5, ..base };
    let b = Sum { list: 2, x: 3 * 5, tie: (2 << 24) | 5, ..base };
    let mut t = Tops::new(5);
    t.offer(a);
    t.offer(b);
    assert_eq!(t.best(0).first().map(|s| s.list), Some(1), "v3: the smaller list first");
    sw::set(sw::TIE52, true);
    let mut t2 = Tops::new(5);
    t2.offer(a);
    t2.offer(b);
    assert_eq!(t2.best(0).first().map(|s| s.list), Some(2), "design 5.2: fewer families on first");
    reset();
}

// ---------------------------------------------------------------- item 1: the exact margin (sizing, refusals, no look-ahead)

fn cube(kind: u8, expiry: u32, lot: u32) -> Cube {
    let d = day();
    let mut c = Cube { ymd: ymd_of(d), day: d, ..Cube::default() };
    c.unds.push(Und { name: "SBIN".to_owned(), kind, spot_seg: kind, spot_sid: 1, nearest: expiry, next: 0 });
    c.cons.push(Contract { sid: 7, und: 0, kind, right: 0, expiry, lot, strike: 100_000, first_min: 0, first_open: 1_000, symbol: "SBIN-X-CE".to_owned(), n_sec: 0 });
    c.spot = vec![100_000; crate::cal::NM];
    c.close = vec![1_000; crate::cal::NM];
    c.flags = vec![5; crate::cal::NM];
    c.high = vec![1_000; crate::cal::NM];
    c.low = vec![1_000; crate::cal::NM];
    c.vol = vec![500; crate::cal::NM];
    c.oi = vec![0; crate::cal::NM];
    c
}

/// ITEM 1: the lots per slot come from the EXACT margin per lot; the proxy is printed beside; a contract absent from the table (a new
/// listing) or a stock without its exposure rate is REFUSED by name (never sized by a guess); a stock without a C_VAR1 row is NOT refused
/// under the exact margin (the proxy is only a reading).
#[test]
fn v31_exact_margin_sizes_lots_and_refuses() {
    let d = day();
    let c = cube(1, ymd_of(d + 7), 50);
    let mut var = std::collections::BTreeMap::new();
    var.insert("SBIN".to_owned(), crate::margin::VarRow { var_x100: 1_200, elm_x100: 350 });
    let t = 36_000_u32;
    let proxy = size_event_mx(&c, 0, t, &var, true, None);
    let mx = MarginAt { lot_exact: 13_000_000, ..MarginAt::default() }; // Rs 1.3 L a lot
    let ex = size_event_mx(&c, 0, t, &var, true, Some(&mx));
    assert_eq!(ex.margin_lot, 13_000_000);
    assert_eq!(ex.margin_lot_proxy, proxy.margin_lot);
    // N 5 (slot Rs 4 L): 3 lots; N 40 (slot Rs 50,000): 0 lots (margin above the slot: refused at the account, margin_over_slot)
    assert_eq!(ex.lots, [3, 1, 0, 0]);
    let refused = MarginAt { refuse: Some(Refuse::NoSpanRow), ..MarginAt::default() };
    assert_eq!(size_event_mx(&c, 0, t, &var, true, Some(&refused)).refuse, Refuse::NoSpanRow);
    let no_elm = MarginAt { refuse: Some(Refuse::NoElmRate), ..MarginAt::default() };
    assert_eq!(size_event_mx(&c, 0, t, &var, true, Some(&no_elm)).refuse, Refuse::NoElmRate);
    // renamed / absent from C_VAR1: the proxy cannot be read, the exact margin still sizes the event
    let empty = std::collections::BTreeMap::new();
    assert_eq!(size_event_mx(&c, 0, t, &empty, true, None).refuse, Refuse::NoMarginRow);
    assert_eq!(size_event_mx(&c, 0, t, &empty, true, Some(&mx)).refuse, Refuse::None);
}

fn snap_with(rows: &[&str]) -> Snapshot {
    let mut t = String::from("date\tund\tseg\texpiry\tright\tstrike\tfile_price\tund_file_price\tspan_unit_short\tscan_unit\tworst_scenario\tsom_unit\telm_rate_pct\telm_class\telm_oth_pct\telm_otm_pct\tlot\tdelta\tseries_price_scan\tfile\tfile_created\tael_file\n");
    for r in rows {
        t.push_str(r);
        t.push('\n');
    }
    Snapshot::parse(t.as_bytes()).unwrap()
}

/// ITEM 1 (no look-ahead): a decision uses the latest table published STRICTLY before T: a table published at T itself is not used; a
/// begin-of-day table is in force from the session start.
#[test]
fn v31_snapshot_strictly_before_t() {
    let d = day();
    let bod = snap_with(&["2026-11-04\tSBIN\tstk\t20261111\tC\t1000\t10\t1000\t100\t100\t11\t0\t3.5\tOTH\t3.5\t5.25\t\t0.5\t142\tnsccl.20261104.i1\tx\tael"]);
    let i2 = snap_with(&["2026-11-04\tSBIN\tstk\t20261111\tC\t1000\t20\t1000\t200\t200\t11\t0\t3.5\tOTH\t3.5\t5.25\t\t0.5\t142\tnsccl.20261104.i2\tx\tael"]);
    let eff = at_sod(d, 11 * 3_600 + 7 * 60);
    let dm = DayM { day: d, fallback_from: None, snaps: vec![(i64::MIN, "nsccl.20261104.i1".to_owned(), bod), (eff, "nsccl.20261104.i2".to_owned(), i2)] };
    let c = cube(1, 2026_11_11, 50);
    let mut lat = crate::mtable::Lat::default();
    let cm: ConM = crate::mtable::resolve(&dm, &c, &mut lat);
    let at = |t: i64| exact_at(&dm, &cm, 0, t, 100_000, 50, ymd_of(d), None);
    assert_eq!(at(eff).snap, 0, "a table published at T itself is not used (strictly before)");
    assert_eq!(at(eff + 1_000_000).snap, 1);
    assert_eq!(at(at_sod(d, OPEN_S)).snap, 0, "the begin-of-day table is in force from the open");
    // SPAN 100 x 50 + premium 10 x 50 + ELM 3.5% x 1,000 x 50 (OTH: the CALL at 1,000 with the spot at 1,000 is not OTM)
    assert_eq!(at(eff).lot_exact, 500_000 + 50_000 + 175_000);
    // a contract not in the table (a new listing): refused and counted
    let mut c2 = c.clone();
    if let Some(k) = c2.cons.get_mut(0) {
        k.strike = 105_000;
    }
    let cm2 = crate::mtable::resolve(&dm, &c2, &mut lat);
    assert_eq!(exact_at(&dm, &cm2, 0, eff, 100_000, 50, ymd_of(d), None).refuse, Some(Refuse::NoSpanRow));
}

/// EXTREME (addendum 2): an NSE table missing for a day falls back to the latest EARLIER table, labelled; a corrupt or partial table (sha256
/// not the manifest's) is refused, never read.
#[test]
fn v31_missing_day_falls_back_and_corrupt_table_refused() {
    let d = day();
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let m = Manifest { rows: vec![SnapMeta { session: d - 2, label: "nsccl.A.i1".to_owned(), bod: true, eff_us: i64::MIN, path: p.clone(), sha: "0".repeat(64) }, SnapMeta { session: d - 1, label: "nsccl.B.i1".to_owned(), bod: true, eff_us: i64::MIN, path: p, sha: "0".repeat(64) }], path: String::new(), sha: String::new() };
    let (v, fb) = m.of_day(d);
    assert_eq!((v.len(), v.first().map(|r| r.label.as_str()), fb), (1, Some("nsccl.B.i1"), Some(d - 1)), "the latest earlier session's table, labelled");
    let (v2, fb2) = m.of_day(d - 1);
    assert_eq!((v2.len(), fb2), (1, None));
    assert!(m.of_day(d - 5).0.is_empty(), "no earlier table at all: nothing (refused at the event: no_margin_file)");
    let e = crate::mtable::load_day(&m, d, true).err().unwrap_or_default();
    assert!(e.contains("sha256"), "a table whose sha256 is not the manifest's is refused: {e}");
}

/// MATRIX X63: the day index is 16 bits: an account walks across session 300 (v2.1's u8 tagged it day 44 and its day counters stopped at
/// 255) and counts every day.
#[test]
fn v31_day_index_over_255_sessions() {
    reset();
    let ev = [(36_000_u32, 1_000_000_i64, 37_000, 500_i32, 1_u32, 1_i64), (36_000, 1_000_000, 37_000, 700, 1, 1)];
    let (mut evs, ex, tab) = fixture(&ev, &|_, _| 0);
    evs[0].di = 10;
    evs[1].di = 300;
    let list: Vec<u32> = vec![0, 1];
    let keys = list_keys(&evs, &list);
    assert!(keys[1] > keys[0] && keys[1] == 300 * 86_400 + 36_000, "the key of session 300 does not wrap");
    let cs = CSet { rules: vec![(false, 0)], sl: 0 };
    let (s, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 0, None, true, false, &[], 0, 301);
    assert_eq!((s.trades, s.days_traded, s.days_pos, s.ndays, s.net), (2, 2, 2, 301, 1_200));
    // the u32 event key holds every session of 2000-2099 (36,525 days): 36,524 x 86,400 + 86,399 < 2^32
    assert!(36_524_u64 * 86_400 + 86_399 < u64::from(u32::MAX));
}

// ---------------------------------------------------------------- decision (c): the jump stress; D3

/// DECISION (c): the stress table's drawdown groups closes of one second (the same reading as item 5).
#[test]
fn v31_stress_table_drawdown_grouped() {
    let mut v = vec![(5, -100), (5, 100), (9, -30), (12, 10)];
    assert_eq!(crate::report3::dd_grouped_pub(&mut v), 30);
}

/// check-build-v3 D3: the day-major mode re-walks every flagged account-row exactly: the composed pick of each window row equals the whole
/// mode's (which re-walks inside the sweep), on the window fixture whose slack check fires.
#[test]
fn v31_d3_rewalk_equals_fresh_walk() {
    reset();
    // the wf.rs fixture: 9 one-day pieces, a -Rs 15 L trade in piece 2 makes the cash rule bite in the continuous walk only
    let mut evs: Vec<SEv> = Vec::new();
    let mut ex: Vec<EvX> = Vec::new();
    let mut tab = Tab3::default();
    let of_day: Vec<u8> = (0..9).collect();
    let nets = [[50_000_000_i32, -70_000_000, 1_000, 2_000], [-20_000, 30_000, -40_000, 5_000], [-150_000_000, 10_000, 10_000, 10_000], [7_000, -1_000, 3_000, -9_000], [100, 200, -300, 400], [-5_000, -5_000, 60_000, 1], [9_000, 9_000, -1, -1], [-80_000_000, 20_000, 30_000, -10_000], [1_000, 2_000, 3_000, 4_000]];
    for d in 0..9_u8 {
        for k in 0..4_u32 {
            let t = 36_000 + k * 600;
            let et = EvT { refuse: Refuse::None, lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot: 60_000_000, filled: [true; NL], ..EvT::default() };
            let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap(), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1: 0 };
            for _ in 0..NL {
                let mut a = [ABSENT; NR];
                a[0] = NEVER3;
                tab.anc.push(a);
                tab.os.push([0; NR]);
                tab.obase.push(u32::try_from(tab.outs.len()).unwrap());
                tab.outs.push(Out { net: nets[usize::from(d)][k as usize], exit: off(i64::from(t) + 300), flag: 1, ..Out::default() });
            }
            evs.push(SEv { di: u16::from(d), ci: k + 10 * u32::from(d), t, skey: k + 10 * u32::from(d), tab: et });
            ex.push(x);
        }
    }
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let cs = CSet { rules: vec![(false, 0)], sl: 0 };
    let (_, pc, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 0, None, true, false, &of_day, crate::wf::NPIECE, 9);
    let mut flagged = 0;
    for row in crate::wf::rows21() {
        if crate::wf::fresh_equal(&pc.p, row.learn.0, row.test.1) {
            continue;
        }
        flagged += 1;
        // the day-major re-walk: a fresh account from the row's first learning day, walked day by day (walk_day), its pieces composed
        let mut ac = crate::kern::Acc::new();
        let mut pcs = crate::kern::Pieces::new(&of_day, crate::wf::NPIECE);
        for d in row.learn.0..9 {
            let (lo, hi) = (u32::try_from(d).unwrap() * 86_400, u32::try_from(d + 1).unwrap() * 86_400);
            let (a, b) = (keys.partition_point(|&k| k < lo), keys.partition_point(|&k| k < hi));
            let _ = crate::kern::walk_day(&evs, &ex, &tab, &list[a..b], &keys[a..b], &cs, 0, 0, None, true, true, &mut ac, &mut pcs, &of_day);
        }
        // the whole mode's re-walk (report3 stage_b_whole's closure)
        let sub: Vec<u32> = list.iter().copied().filter(|&i| usize::from(evs[i as usize].di) >= row.learn.0).collect();
        let sk = list_keys(&evs, &sub);
        let (_, fp, _) = walk_account(&evs, &ex, &tab, &sub, &sk, &cs, 0, 0, None, true, true, &of_day, crate::wf::NPIECE, 9);
        let (x, y) = (crate::wf::compose(&pcs.p, row.learn.0, row.learn.1), crate::wf::compose(&fp.p, row.learn.0, row.learn.1));
        assert_eq!((x.net, x.mdd, x.trades, x.all_pos), (y.net, y.mdd, y.trades, y.all_pos), "row {}", row.id);
        let (x2, y2) = (crate::wf::compose(&pcs.p, row.test.0, row.test.1), crate::wf::compose(&fp.p, row.test.0, row.test.1));
        assert_eq!((x2.net, x2.mdd, x2.trades, x2.all_pos), (y2.net, y2.mdd, y2.trades, y2.all_pos), "row {} test", row.id);
    }
    assert!(flagged > 0, "the fixture's slack check fires");
}

/// The 15:30 order end never moves the decision instants or the 15:10 square-off itself (S14 / B1 unchanged).
#[test]
fn v31_square_off_still_1510() {
    reset();
    sw::set(sw::END_1530, true);
    let d = day();
    let t = 36_000_i64;
    let rows = vec![sec(t - 1, 10_000, 10_000, 1_000), sec(t, 10_000, 9_900, 1_000), sec(SQ_S - 1, 9_000, 9_000, 1_000), sec(SQ_S, 9_500, 9_400, 1_000), sec(SQ_S + 1, 9_600, 9_500, 1_000)];
    let dt = DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let dl = dial().unwrap();
    let mut p = record_base(&tape, d, t, 50, 50, &dl).ok().unwrap();
    let r = trip(&mut p, &tape, d, 50, 4, 50, None, &dl).unwrap();
    assert_eq!((r.sq_units, r.unclosed), (50, 0), "the square-off from 15:10:00 fills at its first block");
    assert_eq!(r.buy_value, 50 * 9_600, "the block [15:10:00, 15:10:02)'s highest HIGH");
    let _ = (ACCOUNT, Row::default());
    reset();
}

// ---------------------------------------------------------------- margin rule v2 (addendum 9); matrix X02, X05; addendum 7

/// ADDENDUM 9 (margin rule v2): lots = floor((slot - Rs 20) / margin per lot) and the Rs 20 is blocked with the lots (one order).
#[test]
fn v31_margin_rule_v2_sizing_addon() {
    let d = day();
    let c = cube(1, ymd_of(d + 7), 50);
    let var = std::collections::BTreeMap::new();
    let t = 36_000_u32;
    // Rs 2,00,000.00 a lot exactly: N 10 (slot Rs 2 L) fits 1 lot under rule v1 but 0 under rule v2 (the Rs 20 does not fit)
    let v1 = MarginAt { lot_exact: 20_000_000, ..MarginAt::default() };
    let v2 = MarginAt { lot_exact: 20_000_000, addon: common_fill::margin::ORDER_ADDON_PAISA, ..MarginAt::default() };
    let e1 = size_event_mx(&c, 0, t, &var, true, Some(&v1));
    let e2 = size_event_mx(&c, 0, t, &var, true, Some(&v2));
    assert_eq!((e1.lots, e1.addon), ([2, 1, 0, 0], 0));
    assert_eq!((e2.lots, e2.addon), ([1, 0, 0, 0], 2_000), "N 5: floor((Rs 4 L - Rs 20) / Rs 2 L) = 1; N 10: 0");
}

/// MATRIX X02: a never-closed valuation window without a print is NEVER valued at Rs 0 (that would turn a loss into a full-premium win): the
/// higher of the last print at or before the exit decision and the entry window, counted.
#[test]
fn v31_never_closed_empty_window_never_zero() {
    reset();
    sw::set(sw::END_1530, true);
    sw::set(sw::VALUE_FROM_EXIT, true);
    let d = day();
    let t = 36_000_i64;
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000)];
    for k in 0..7_200 {
        rows.push(sec(t + k, 12_000, 11_900, 1));
    }
    let dt = DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    // the exit decision at 13:00: nothing prints after 12:00
    let (v, _, fb) = never_closed_value(&tape, d, at_sod(d, t), at_sod(d, 13 * 3_600));
    assert!(fb, "the empty window is counted");
    assert_eq!(v, 12_000, "never Rs 0: the entry window's highest HIGH / the last print before the decision");
    reset();
}

/// ADDENDUM 7 / checklist 9: the own 15:10 square-off is one more own order (Rs 20) without Dhan's square-off fee; Dhan's fee only when
/// units stay open (never closed).
#[test]
fn v31_own_squareoff_orders_and_fee() {
    reset();
    assert_eq!(crate::trade::buy_orders(0, 50, 0), (0, true), "v2.1: the square-off is Dhan's, with its fee");
    sw::set(sw::OWN_SQUAREOFF, true);
    assert_eq!(crate::trade::buy_orders(0, 50, 0), (1, false));
    assert_eq!(crate::trade::buy_orders(50, 0, 0), (1, false));
    assert_eq!(crate::trade::buy_orders(25, 25, 0), (2, false));
    assert_eq!(crate::trade::buy_orders(0, 0, 50), (0, true), "never-closed units: Dhan's auto square-off fee");
    reset();
}

/// MATRIX X05 / X06 through the engine's trip: a stop whose first buy-back blocks are past the protection limit (the price jumped and keeps
/// rising) is re-sent each block and fills; v2.1's single limit leaves it to the 15:10 square-off.
#[test]
fn v31_resend_stop_buyback_fills() {
    reset();
    let d = day();
    let t = 36_000_i64;
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000), sec(t, 10_000, 9_950, 1_000), sec(t + 1, 10_000, 9_950, 1_000)];
    // at 10:00:10 the option jumps to Rs 125 and climbs 4% a second; the own buy-back is anchored at 10:00:11 (its limit from the 125 print)
    let mut px = 12_500_i64;
    rows.push(sec(t + 10, i32::try_from(px).unwrap(), i32::try_from(px).unwrap(), 1_000));
    for k in 11..60 {
        px = px * 104 / 100 + if k == 11 { 2_000 } else { 0 };
        rows.push(sec(t + k, i32::try_from(px).unwrap(), i32::try_from(px).unwrap(), 1_000));
    }
    rows.push(sec(SQ_S, 30_000, 30_000, 5_000));
    rows.push(sec(SQ_S + 1, 30_000, 30_000, 5_000));
    let dl = dial().unwrap();
    for on in [false, true] {
        sw::set(sw::RESEND, on);
        let dt = DayTape::new(d, &rows, 1).unwrap();
        let tape = dt.tape().unwrap();
        let mut p = record_base(&tape, d, t, 50, 50, &dl).ok().unwrap();
        let r = trip(&mut p, &tape, d, 50, 4, 50, Some(t + 11), &dl).unwrap();
        if on {
            assert_eq!(r.bb_units, 50, "re-sent: bought back near the jump");
        } else {
            assert_eq!(r.bb_units, 0, "v2.1: the one limit was passed at once; the short rides to 15:10");
            assert_eq!(r.sq_units, 50);
        }
    }
    reset();
}

// ---------------------------------------------------------------- check-build-v3 D1 / D3 on the driver itself

fn tiny_seg() -> crate::report3::Seg3 {
    let mut sr = crate::report3::Seg3 { ngroup: crate::report3::NGROUP, ..crate::report3::Seg3::default() };
    sr.evs = vec![SEv { di: 0, ci: 1, t: 36_000, skey: 1, tab: EvT::default() }, SEv { di: 0, ci: 2, t: 36_060, skey: 2, tab: EvT::default() }];
    sr.ex = vec![EvX { nq: 1, mark1: 0, ..EvX::default() }, EvX { nq: 1, mark1: 0, ..EvX::default() }];
    sr.ul_evs = vec![vec![0], vec![1]];
    sr.uls = vec![(0, 0, u8::MAX), (0, 0, u8::MAX)];
    sr.tops = (0..crate::report3::NGROUP).map(|_| Tops::new(5)).collect();
    sr
}

/// D1 (check-build-v3): after the day-major sweep the stage A rows of every event outside the picks' lists are DROPPED (no detail may read
/// a table the sweep threw away); the picks' events are kept for the rebuild.
#[test]
fn v31_d1_drops_rows_outside_the_picks() {
    reset();
    let mut sr = tiny_seg();
    // THE pick (group 6, view S) uses list 0 (event 0)
    if let Some(t) = sr.tops.get_mut(6) {
        t.offer(Sum { list: 0, trades: 1, net: 1, ndays: 1, days_pos: 1, ..Sum::default() });
    }
    let mut store = crate::report3::Store::new(std::path::Path::new("/nonexistent"), &[], true).unwrap();
    let ma = crate::stage::MenuA::default();
    let dl = dial().unwrap();
    let mut log = String::new();
    crate::report3::d1_rebuild(&mut sr, &mut store, &std::collections::BTreeMap::new(), &dl, &ma, 0, false, 1, false, &mut log).unwrap();
    assert_eq!(sr.ex[1].nq, 0, "event 1 is in no pick's list: its stale rows are dropped");
    assert_eq!(sr.ex[0].nq, 1, "event 0 belongs to THE pick's list: kept for the rebuild");
}

/// D3 (check-build-v3): every account-row the day-major slack check flagged is re-walked by the second pass and offered: nothing stays
/// PENDING.
#[test]
fn v31_d3_leaves_nothing_pending() {
    reset();
    let mut sr = tiny_seg();
    let rows = crate::wf::rows21();
    let ri = rows.iter().position(|r| r.learn.0 == 1).unwrap();
    sr.win = rows.iter().map(|r| crate::report3::WinRes { row: Some(r.clone()), run: true, ..crate::report3::WinRes::default() }).collect();
    if let Some(w) = sr.win.get_mut(ri) {
        w.pending = 1;
    }
    sr.flagged = vec![crate::report3::Flag { row: u8::try_from(ri).unwrap(), list: 0, si: 0, l: 0, ni: 0, sm: Sum { trades: 1, ..Sum::default() } }];
    sr.piece_of_day = vec![1];
    let mut store = crate::report3::Store::new(std::path::Path::new("/nonexistent"), &[], true).unwrap();
    let ma = crate::stage::MenuA::default();
    let dl = dial().unwrap();
    let menu_sets = crate::menu::Sets::default();
    let mut log = String::new();
    let cs = vec![CSet { rules: vec![(false, 0)], sl: 0 }];
    crate::report3::d3_rewalk(&mut sr, &mut store, &std::collections::BTreeMap::new(), &dl, &ma, &menu_sets, &[0], &cs, 0, false, 1, &mut log).unwrap();
    assert_eq!((sr.win[ri].pending, sr.win[ri].rewalked), (0, 1), "the flagged account-row was re-walked, none left pending");
}
