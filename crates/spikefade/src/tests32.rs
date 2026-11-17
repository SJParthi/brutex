//! v3.2 NAMED TESTS (MAIN-RULINGS-ENGINE-v22 addenda 8b, 10, 11, 12-16; the v3.1 check's M1-M4 / S1-S8). Every case is synthetic and
//! integer (the Greeks case aside: the app's f64 crate); each test names the rule it guards, and each named mutant (Cargo features
//! m_v32_*) must make its test FAIL. The switches are per test thread (`sw`); every test starts from v3.1's setting and sets only what it
//! tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_precision_loss, clippy::float_cmp)]

use std::collections::BTreeSet;

use common_fill::margin::Snapshot;

use crate::acct::Sum;
use crate::cal::{SQ_S, at_sod, days_from_civil, ymd_of};
use crate::cube::{Contract, Cube, Sec, Und};
use crate::mtable::{ConM, DayM, exact_at};
use crate::sw;
use crate::trade::{DayTape, dial, record_base, trip};

fn day() -> i64 {
    days_from_civil(2026, 11, 4)
}

fn sec(s: i64, h: i32, l: i32, v: u32) -> Sec {
    Sec { sod: u32::try_from(s).unwrap_or(0), high: h, low: l, vol: v }
}

/// Every switch back to v3 (each test sets only what it tests).
fn reset() {
    for s in 0..sw::COUNT {
        sw::set(s, false);
    }
}

/// A tape where a 2-lot (lot 50) sale fills 50 units a second for 2 seconds and a buy-back at 10:00:30 fills 50 units a second for 2
/// seconds: with 1-second blocks the sale fills in 2 blocks and the buy-back in 2 blocks.
fn two_block_rows(t: i64) -> Vec<Sec> {
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 50)];
    rows.push(sec(t, 10_000, 9_900, 50));
    rows.push(sec(t + 1, 10_000, 9_800, 50));
    // a print 1 s before the buy-back's send (never filled: before the order), then one print a second: two filling blocks of 50
    rows.push(sec(t + 29, 9_000, 9_000, 50));
    rows.push(sec(t + 30, 9_000, 9_000, 50));
    rows.push(sec(t + 31, 9_100, 9_000, 50));
    rows.push(sec(SQ_S, 9_000, 9_000, 5_000));
    rows
}

/// ADDENDUM 10 (THE NET DEFINITION; check M1 / M4): under the switch every filling block x every freeze slice is one executed order (Rs 20
/// + GST) in the charges AND in the +2% gate's re-price. A 100-unit sale in 2 blocks and a 100-unit own buy-back in 2 blocks: 2 + 2 orders
/// (v3.1: 1 + 1); with a freeze slice of ONE lot (50 units) still 2 + 2 (each block 50 units); the gate re-prices with the same orders.
#[test]
fn v32_a10_orders_in_charges_and_gate() {
    reset();
    let d = day();
    let t = 36_000_i64;
    let rows = two_block_rows(t);
    let dl = dial().unwrap();
    let dt = DayTape::new(d, &rows, 1).unwrap();
    let tape = dt.tape().unwrap();
    // v3.1's count
    let mut p = record_base(&tape, d, t, 50, 100, &dl).ok().unwrap();
    let r31 = trip(&mut p, &tape, d, 50, 4, 100, Some(t + 30), &dl).unwrap();
    assert_eq!((r31.units, r31.bb_units), (100, 100));
    assert_eq!(r31.orders_a10(), None, "switch off: v3.1's order count");
    // addendum 10
    sw::set(sw::ORDERS_A10, true);
    let mut p = record_base(&tape, d, t, 50, 100, &dl).ok().unwrap();
    p.slice = 0;
    let r = trip(&mut p, &tape, d, 50, 4, 100, Some(t + 30), &dl).unwrap();
    assert_eq!((r.sale_blocks, r.bb_blocks), (2, 2));
    assert_eq!(r.orders_a10(), Some((2, 2, 0)), "every filling block is an executed order");
    let (c, _, cs, _) = crate::trade::charges_orders_n(d, r.units, r.buy_value, r.sold_value, 2, 2, 0).unwrap();
    assert_eq!(r.charges, c, "the trip's charges count every block");
    assert!(r.charges - r31.charges >= 2 * 2_000, "two more orders: at least Rs 40 more brokerage (+ GST)");
    assert_eq!(r.net, r.sold_value - r.buy_value - c);
    assert!(cs > r31.charges_sell, "the sale side carries its 2 orders");
    // the +2% gate re-prices with the SAME orders
    let m = crate::stress::Money { day: d, units: r.units, sold: r.sold_value, bought: r.buy_value, own_orders: 1, broker: false, orders: r.orders_a10() };
    let g = crate::stress::stressed(&m, 200, true).unwrap();
    let sold2 = r.sold_value * 9_800 / 10_000;
    let bought2 = (r.buy_value * 10_200 + 9_999) / 10_000;
    let (c2, _, _, _) = crate::trade::charges_orders_n(d, r.units, bought2, sold2, 2, 2, 0).unwrap();
    assert_eq!(g, sold2 - bought2 - c2, "the gate counts the executed orders of addendum 10");
    // a freeze slice of one lot: each 50-unit block is one slice; a slice of HALF the block would double the orders (crate O1)
    let mut p = record_base(&tape, d, t, 50, 100, &dl).ok().unwrap();
    p.slice = 50;
    let r1 = trip(&mut p, &tape, d, 50, 4, 100, Some(t + 30), &dl).unwrap();
    assert_eq!(r1.orders_a10(), Some((2, 2, 0)));
    reset();
}

/// ADDENDUM 10 + checklist 12: an order above NSE's freeze quantity is SLICED and every slice of every filling block is an order: a 4-lot
/// (200-unit) block under a 1-lot slice = 4 sale orders.
#[test]
fn v32_freeze_slices_counted_in_the_trip() {
    reset();
    sw::set(sw::ORDERS_A10, true);
    let d = day();
    let t = 36_000_i64;
    let rows = vec![sec(t - 1, 10_000, 10_000, 200), sec(t, 10_000, 9_900, 200), sec(t + 1, 10_000, 9_900, 200), sec(SQ_S, 9_000, 9_000, 5_000), sec(SQ_S + 1, 9_000, 9_000, 5_000)];
    let dl = dial().unwrap();
    let dt = DayTape::new(d, &rows, 1).unwrap();
    let tape = dt.tape().unwrap();
    let mut p = record_base(&tape, d, t, 50, 200, &dl).ok().unwrap();
    p.slice = 50;
    let r = trip(&mut p, &tape, d, 50, 4, 200, None, &dl).unwrap();
    assert_eq!(r.units, 200);
    assert_eq!(r.sale_blocks, 1, "the whole sale in one block");
    assert_eq!(r.sale_orders, 4, "4 lots under a 1-lot freeze slice: 4 executed orders");
    reset();
}

/// CHECKLIST 12: the DATED freeze limit of a session is the latest NSE publication effective on or before it (never a later file for an
/// earlier day); a stock session before the first stock file reads the earliest file, labelled undated; slice = whole lots under the
/// freeze (NIFTY 1800 / lot 65 = 27 lots = 1,755 units; BANKNIFTY 900 / 30 = 900).
#[test]
fn v32_freeze_dated_lookup() {
    let dir = std::env::temp_dir().join(format!("sf32-freeze-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let idx = dir.join("FREEZE-TABLE-NIFTY-BANKNIFTY.tsv");
    std::fs::write(&idx, "index\teffective_from\tfreeze_qty_units\tx\n NIFTY\t2026-10-05\t3510\tc\nNIFTY\t2026-11-02\t1800\tc\nBANKNIFTY\t2026-10-05\t1440\tc\nBANKNIFTY\t2026-11-02\t900\tc\n").unwrap();
    let stk = dir.join("qtyfreeze.csv");
    std::fs::write(&stk, "S.No.,SYMBOL    ,VOL_FRZ_QTY    \n1    ,NIFTY     ,1800 \n2    ,SBIN      ,24000\n").unwrap();
    let mut f = crate::freeze::Freeze::default();
    f.add(idx.to_str().unwrap()).unwrap();
    f.add(&format!("{}@2026-11-02", stk.to_str().unwrap())).unwrap();
    let dd = |y, m, d| days_from_civil(y, m, d);
    assert_eq!(f.units("NIFTY", dd(2026, 10, 20)), Some((3510, true)), "the October circular for an October session");
    assert_eq!(f.units("NIFTY", dd(2026, 11, 3)), Some((1800, true)));
    assert_eq!(f.units("BANKNIFTY", dd(2026, 11, 2)), Some((900, true)), "effective ON the session");
    assert_eq!(f.units("SBIN", dd(2026, 11, 4)), Some((24_000, true)));
    assert_eq!(f.units("SBIN", dd(2026, 10, 1)), Some((24_000, false)), "before the first stock file: the earliest file, UNDATED (labelled)");
    assert_eq!(f.units("NEWCO", dd(2026, 11, 4)), None, "no limit known");
    assert_eq!(crate::freeze::slice_of(1_800, 65), 1_755);
    assert_eq!(crate::freeze::slice_of(900, 30), 900);
    assert_eq!(crate::freeze::slice_of(10, 25), 25, "a lot alone above the freeze: one lot an order (counted)");
    let _ = std::fs::remove_file(&idx);
    let _ = std::fs::remove_file(&stk);
    let _ = std::fs::remove_dir(&dir);
}

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

/// CHECKLIST 10: a stock option of an underlying in NSE's F&O ban list that session is REFUSED (fno_ban); the same stock the next session
/// (not listed) is sized; an index option is never touched by the stock ban list.
#[test]
fn v32_ban_day_refused() {
    reset();
    let d = day();
    let dir = std::env::temp_dir().join(format!("sf32-ban-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (y, m, dd) = crate::cal::civil(d);
    let (y2, m2, d2) = crate::cal::civil(d + 1);
    std::fs::write(dir.join(format!("fo_secban_{dd:02}{m:02}{y}.csv")), "Securities in Ban For Trade Date 04-NOV-2026:\n1,SAIL\n").unwrap();
    std::fs::write(dir.join(format!("fo_secban_{d2:02}{m2:02}{y2}.csv")), "Securities in Ban For Trade Date 05-NOV-2026:\n").unwrap();
    let mut b = crate::ban::Bans::default();
    b.add_dir(&dir).unwrap();
    assert_eq!(b.status(d, "SAIL"), Some(true));
    assert_eq!(b.status(d + 1, "SAIL"), Some(false), "a NIL list the next session");
    assert_eq!(b.status(d + 2, "SAIL"), None, "no file: labelled, never assumed in silence");
    let _ = crate::ban::BANS.set(b);
    sw::set(sw::BAN, true);
    let mut var = std::collections::BTreeMap::new();
    var.insert("SAIL".to_owned(), crate::margin::VarRow { var_x100: 1_200, elm_x100: 350 });
    let c = cube(1, "SAIL", ymd_of(d + 7), 50);
    let e = crate::trade::size_event_mx(&c, 0, 36_000, &var, true, None);
    assert_eq!(e.refuse, crate::trade::Refuse::FnoBan, "a banned underlying's stock option is refused that day");
    let mut c1 = c.clone();
    c1.day = d + 1;
    c1.ymd = ymd_of(d + 1);
    assert_eq!(crate::trade::size_event_mx(&c1, 0, 36_000, &var, true, None).refuse, crate::trade::Refuse::None);
    sw::set(sw::BAN, false);
    assert_eq!(crate::trade::size_event_mx(&c, 0, 36_000, &var, true, None).refuse, crate::trade::Refuse::None, "switch off: v3.1");
    for f in std::fs::read_dir(&dir).unwrap().flatten() {
        let _ = std::fs::remove_file(f.path());
    }
    let _ = std::fs::remove_dir(&dir);
    reset();
}

/// ADDENDUM 14 (b)+(c): the last-60-second volume at the headline share (25%) must cover the WHOLE order within F seconds and its buy-back
/// within Fx seconds; a slot count that fails is refused (never half-entered). Lot 50: 5 lots (250 units) need 25% x vol60 x 60 / 60 >=
/// 250, i.e. vol60 >= 1,000 at F = 60 s; 999 fails the 5-lot order but not the 1-lot one.
#[test]
fn v32_liq_entry_gate_covers_whole_order() {
    reset();
    let lots = [5_u16, 2, 1, 0];
    assert_eq!(crate::trade::cover_bits(&lots, 50, 1_000), 0b0111);
    assert_eq!(crate::trade::cover_bits(&lots, 50, 999), 0b0110, "999 units in 60 s cannot cover 250 units at 25%");
    assert_eq!(crate::trade::cover_bits(&lots, 50, 199), 0b0000);
    let mut e = crate::trade::EvT { lots, cover: crate::trade::cover_bits(&lots, 50, 999), ..crate::trade::EvT::default() };
    assert!(e.cover_ok(0), "switch off: v3.1 (no coverage gate)");
    sw::set(sw::LIQ14, true);
    assert!(!e.cover_ok(0), "5 lots not covered: refused");
    assert!(e.cover_ok(1) && e.cover_ok(2));
    // through the kernel: the event is refused with the account unchanged
    e.margin_lot = 1_000_000;
    let ev = crate::acct::SEv { di: 0, ci: 0, t: 36_000, skey: 1, tab: e };
    let x = crate::stage::EvX::default();
    let tab = crate::stage::Tab3::default();
    let cs = crate::kern::CSet::default();
    let mut ac = crate::kern::Acc::new();
    let mut p = vec![crate::kern::Pos::default(); 5];
    let mut pc = crate::kern::Pieces::default();
    let (code, _, _) = crate::kern::step3(&mut ac, &mut p, &ev, &x, &tab, &cs, 0, 0, None, true, &mut pc, 0);
    assert_eq!(code, "liq_cover");
    assert_eq!((ac.nopen, ac.trades, ac.margin_open), (0, 0, 0), "refused, never half-entered");
    reset();
}

/// ADDENDUM 14 (d): a headline needs 0 unfilled exits (units valued, not filled).
#[test]
fn v32_headline_needs_zero_unfilled_exits() {
    reset();
    sw::set(sw::HEADLINE, true);
    let ok = Sum { net_s2: 1, ..Sum::default() };
    let unf = Sum { net_s2: 1, nc_trades: 1, nc_units: 50, ..Sum::default() };
    assert!(crate::report3::headline_gate(true, 1, &unf), "v3.1: never-closed units did not block the headline");
    sw::set(sw::LIQ14, true);
    assert!(crate::report3::headline_gate(true, 1, &ok));
    assert!(!crate::report3::headline_gate(true, 1, &unf), "addendum 14 (d): unfilled exits must be 0 for a headline");
    reset();
}

/// ADDENDUM 14 (a): the top-K-by-volume dial is 5 / 10 / 20 under the switch (v3.1: 10 / 20 / 50).
#[test]
fn v32_topk_by_volume_5_10_20() {
    reset();
    assert_eq!(crate::screen::topn(), [10, 20, 50]);
    sw::set(sw::LIQ14, true);
    assert_eq!(crate::screen::topn(), [5, 10, 20]);
    assert_eq!(crate::read::nliq(), 10, "the P-and-G liquidity values join the dial");
    assert!(crate::read::liq_pass(8, 45, 5) && !crate::read::liq_pass(8, 45, 6) && !crate::read::liq_pass(8, 44, 5));
    reset();
    assert_eq!(crate::read::nliq(), 7);
}

/// ADDENDA 13 / 13b: the previous close comes from the PREVIOUS session's bhavcopy (published the evening before), never the session's own
/// file (published after its close: look-ahead); both numbers are read: ClsPric (NSE's official close) and LastPric (Dhan's number).
#[test]
fn v32_prev_close_from_previous_session() {
    let dir = std::env::temp_dir().join(format!("sf32-bhav-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["BhavCopy_NSE_FO_0_0_0_20261104_F_0000.csv.zip", "BhavCopy_NSE_FO_0_0_0_20261105_F_0000.csv.zip", "fo30OCT2026bhav.csv.zip"] {
        std::fs::write(dir.join(n), b"x").unwrap();
    }
    let mut b = crate::bhav::Bhav::default();
    b.add_dir(&dir).unwrap();
    let d5 = days_from_civil(2026, 11, 5);
    assert_eq!(b.prev_of(d5), Some(days_from_civil(2026, 11, 4)), "the 5 Nov trade reads the 4 Nov file");
    assert_eq!(b.prev_of(days_from_civil(2026, 11, 4)), Some(days_from_civil(2026, 10, 30)), "the old format named too");
    let text = "TradDt,BizDt,Sgmt,Src,FinInstrmTp,FinInstrmId,ISIN,TckrSymb,SctySrs,XpryDt,FininstrmActlXpryDt,StrkPric,OptnTp,FinInstrmNm,OpnPric,HghPric,LwPric,ClsPric,LastPric,PrvsClsgPric\n2026-11-04,2026-11-04,FO,NSE,STO,123737,,PIIND,,2026-11-23,2026-11-23,1900.00,PE,PIIND26NOV1900PE,8.00,8.75,6.30,7.35,7.80,7.80\n2026-11-04,2026-11-04,FO,NSE,STF,1,,PIIND,,2026-11-23,2026-11-23,0,XX,PIINDFUT,1,1,1,9999,9999,1\n";
    let bd = crate::bhav::parse(text, days_from_civil(2026, 11, 4), "t", "s").unwrap();
    assert_eq!(bd.px.get(&("PIIND".to_owned(), 2026_11_23, 190_000, 1)), Some(&(735, 780)), "ClsPric 7.35, LastPric 7.80 (Dhan's 7.80)");
    assert_eq!(bd.rows, 1, "futures rows dropped first");
    for f in std::fs::read_dir(&dir).unwrap().flatten() {
        let _ = std::fs::remove_file(f.path());
    }
    let _ = std::fs::remove_dir(&dir);
}

/// ADDENDUM 16: days to expiry count SESSIONS (weekdays not in NSE's holiday list, plus special sessions), never calendar days: Fri 6 Nov
/// 2026 -> Tue 10 Nov = 2 sessions (Mon, Tue); with Tue a holiday, an expiry moved to Mon 9 Nov = 1; a Sunday special session counts.
#[test]
fn v32_dte_counts_sessions() {
    let dd = |y, m, d| days_from_civil(y, m, d);
    let none = BTreeSet::new();
    assert_eq!(crate::expiry::sessions_between(dd(2026, 11, 6), dd(2026, 11, 10), &none, &none), 2);
    let hol: BTreeSet<i64> = [dd(2026, 11, 10)].into_iter().collect();
    assert_eq!(crate::expiry::sessions_between(dd(2026, 11, 6), dd(2026, 11, 9), &hol, &none), 1);
    let sp: BTreeSet<i64> = [dd(2026, 11, 8)].into_iter().collect();
    assert_eq!(crate::expiry::sessions_between(dd(2026, 11, 6), dd(2026, 11, 9), &hol, &sp), 2, "the Sunday Muhurat session counts");
    assert_eq!(crate::expiry::sessions_between(dd(2026, 11, 9), dd(2026, 11, 9), &hol, &sp), 0, "0 = the expiry day");
    assert_eq!(crate::expiry::bucket(0), 0);
    assert_eq!(crate::expiry::bucket(4), 3);
    assert_eq!(crate::expiry::bucket(9), 4);
    assert_eq!(crate::expiry::weekday(dd(2026, 11, 10)), "Tue");
    // a holiday move: Tue 10 Nov 2026 a holiday, the weekly expiry on Mon 9 Nov is MOVED; a regime change is not a move
    let is_sess = |d: i64| (d + 3).rem_euclid(7) < 5 && d != dd(2026, 11, 10);
    let exps = [2026_11_03_u32, 2026_11_09, 2026_11_17, 2026_11_24];
    assert_eq!(crate::expiry::regular_and_moved(&exps, 2026_11_09, &is_sess), (1, true), "Monday expiry for a Tuesday holiday");
    assert_eq!(crate::expiry::regular_and_moved(&exps, 2026_11_17, &is_sess), (1, false));
    let mixed = [2025_08_07_u32, 2025_08_14, 2025_08_21, 2025_08_28, 2025_09_02];
    assert_eq!(crate::expiry::regular_and_moved(&mixed, 2025_08_28, &is_sess).1, false, "the old Thursday regime is not a holiday move");
    // a MONTHLY expiry moved by a holiday (Tue 24 Nov 2026, Guru Nanak): Mon 23 Nov against NIFTY's weekly regime is MOVED
    let is_sess2 = |d: i64| (d + 3).rem_euclid(7) < 5 && d != dd(2026, 11, 10) && d != dd(2026, 11, 24);
    let nifty = [2026_11_03_u32, 2026_11_09, 2026_11_17, 2026_11_23, 2026_12_01];
    assert_eq!(crate::expiry::regular_and_moved(&nifty, 2026_11_23, &is_sess2), (1, true), "the November monthly on Monday 23 Nov: moved");
    // a weekly series tied between its regular Tuesday and holiday Mondays (Oct 2026): Mon 19 Oct before the Tue 20 Oct holiday is MOVED
    let is_sess3 = |d: i64| (d + 3).rem_euclid(7) < 5 && d != dd(2026, 10, 20) && d != dd(2026, 11, 10) && d != dd(2026, 11, 24);
    let tie = [2026_10_19_u32, 2026_10_27, 2026_11_03, 2026_11_09, 2026_11_17, 2026_11_23];
    assert_eq!(crate::expiry::regular_and_moved(&tie, 2026_10_19, &is_sess3), (1, true), "a tie: the Tuesday holiday still moves the Monday expiry");
    assert_eq!(crate::expiry::regular_and_moved(&tie, 2026_10_27, &is_sess3).1, false, "a regular Tuesday in the tie is not moved");
    // another weekday regime (BANKNIFTY's Wednesdays in 2024) against NIFTY's Thursdays: the next day is a session, never 'moved'
    assert_eq!(crate::expiry::regular_and_moved(&[2024_03_07_u32, 2024_03_14, 2024_03_21, 2024_03_28], 2024_03_13, &is_sess).1, false);
}

/// ADDENDUM 16: the which-expiry dial: nearest (v3.1), next, the current monthly (the last expiry of the nearest's month), busiest (chosen
/// per decision second: None here).
#[test]
fn v32_expiry_pick_dial() {
    use std::sync::atomic::Ordering;
    let exps = [2026_11_03_u32, 2026_11_10, 2026_11_17, 2026_11_24, 2026_12_29];
    assert_eq!(crate::expiry::monthly_of(&exps, 2026_11_03), Some(2026_11_24));
    crate::expiry::PICK.store(1, Ordering::Relaxed);
    assert_eq!(crate::expiry::pick_of(2026_11_03, 2026_11_10, &exps), Some(2026_11_10), "next");
    crate::expiry::PICK.store(2, Ordering::Relaxed);
    assert_eq!(crate::expiry::pick_of(2026_11_03, 2026_11_10, &exps), Some(2026_11_24), "the current monthly");
    crate::expiry::PICK.store(3, Ordering::Relaxed);
    assert_eq!(crate::expiry::pick_of(2026_11_03, 2026_11_10, &exps), None, "busiest: per decision second");
    assert_eq!(crate::expiry::candidates(2026_11_03, 2026_11_10, &exps), vec![2026_11_03, 2026_11_10, 2026_11_24]);
    crate::expiry::PICK.store(0, Ordering::Relaxed);
    assert_eq!(crate::expiry::pick_of(2026_11_03, 2026_11_10, &exps), Some(2026_11_03));
}

/// ADDENDUM 15: a month passes at Rs 2,00,000 net or more (inclusive); the kernel's month and week accumulators count months seen / passing
/// / the worst, weeks seen / positive / the worst.
#[test]
fn v32_month_verdict_2_lakh_inclusive() {
    assert!(crate::kern::month_ok(20_000_000), "exactly Rs 2,00,000 passes");
    assert!(!crate::kern::month_ok(19_999_999));
    let mut ac = crate::kern::Acc::new();
    // month 0: two weeks (+1.5 L, +0.5 L) = 2.0 L; month 1: one week -10,000
    crate::kern::verdict_ids(&mut ac, 0, 0, 15_000_000);
    crate::kern::verdict_ids(&mut ac, 0, 1, 5_000_000);
    crate::kern::verdict_ids(&mut ac, 1, 2, -1_000_000);
    crate::kern::close_month(&mut ac);
    assert_eq!((ac.mo_seen, ac.mo_ok, ac.mo_min), (2, 1, -1_000_000));
    assert_eq!((ac.wk_seen, ac.wk_pos, ac.wk_min), (3, 2, -1_000_000));
}

/// ADDENDUM 15 (s26 portfolio layer): combos share ONE Rs 20 L account at every instant: two members each wanting Rs 15 L at the same
/// instant: one is taken, the other refused by the shared cap; after the first closes, a later trade fits.
#[test]
fn v32_portfolio_shared_cap() {
    let d = day();
    let a: Vec<crate::v32::PTrade> = vec![(at_sod(d, 36_000), at_sod(d, 37_000), 150_000_000, 1_000_000, d, 1, 11)];
    let b: Vec<crate::v32::PTrade> = vec![(at_sod(d, 36_000), at_sod(d, 36_500), 150_000_000, 2_000_000, d, 2, 12), (at_sod(d, 38_000), at_sod(d, 39_000), 150_000_000, 500_000, d, 3, 13)];
    let months = crate::v32::months_of(&[d]);
    let (net, n, refused, ruled, _, mo) = crate::v32::portfolio(&[&a, &b], &months);
    assert_eq!((n, refused, ruled), (2, 1, 0), "the second Rs 15 L at the same instant passes the shared Rs 20 L cap: refused");
    // same entry instant: the trade known first is taken first (deterministic): b's first trade, then b's second after it closed
    assert_eq!(net, 2_000_000 + 500_000);
    assert_eq!(mo.values().copied().sum::<i64>(), net);
    // ONE account: the same trade from two combos (or another strike of the same underlying + side + expiry) is taken once
    let c1: Vec<crate::v32::PTrade> = vec![(at_sod(d, 36_000), at_sod(d, 37_000), 1_000_000, 100_000, d, 7, 70)];
    let c2: Vec<crate::v32::PTrade> = vec![(at_sod(d, 36_000), at_sod(d, 37_000), 1_000_000, 100_000, d, 7, 70), (at_sod(d, 36_100), at_sod(d, 36_900), 1_000_000, 50_000, d, 8, 70)];
    let (net2, n2, cap2, ruled2, _, _) = crate::v32::portfolio(&[&c1, &c2], &months);
    assert_eq!((n2, cap2, ruled2, net2), (1, 0, 2, 100_000), "the duplicate and the same-key strike are refused by the one-account rules");
}

/// ADDENDA 8 / 8b: the Greeks inputs "as /db shows them": the 1-minute bar holding the last second before the decision, the tenor from the
/// bar's STAMP to the expiry day's derivatives close (15:30; 15:40 from 2026-08-03) over 365 days; the app's crate solves the IV back.
#[test]
fn v32_greeks_app_conversions() {
    let d = day();
    let c = cube(0, "NIFTY", ymd_of(d + 1), 65);
    // decision at 10:00:00: the bar 09:59:00-10:00:00 (slot 44), stamp 09:59:00; expiry tomorrow at 15:40 (after 2026-08-03)
    let (m, stamp, prem, spot, tenor) = crate::gk::inputs_at(&c, 0, 36_000).unwrap();
    assert_eq!((m, stamp, prem, spot), (44, 9 * 3_600 + 59 * 60, 1_000, 100_000));
    assert_eq!(tenor, 86_400 + 15 * 3_600 + 40 * 60 - (9 * 3_600 + 59 * 60), "from the bar's stamp to 15:40 of the expiry day");
    assert_eq!(crate::gk::close_of_expiry(days_from_civil(2026, 8, 2)), 15 * 3_600 + 30 * 60);
    // the app's continuous rate (pull::riskfree::continuous) and an IV solved back from the app's own price
    let r = crate::gk::continuous(55_199);
    assert!((r - (0.055_199_f64 * 91.0 / 365.0).ln_1p() / (91.0 / 365.0)).abs() < 1e-15);
    let ct = greeks::Contract { spot: 2_500_000.0, strike: 2_500_000.0, years_to_expiry: 7.0 / 365.0, rate: r, carry: 0.0 };
    let px = ct.price(0.15, greeks::OptionKind::Call).unwrap();
    let g = crate::gk::bar_greeks(2_500_000, 2_500_000, px.round() as i64, 7 * 86_400, true, 55_199, -65);
    assert!(g.ok, "solved: {}", g.refusal);
    assert!((g.iv - 0.15).abs() < 1e-3, "IV back {}", g.iv);
    assert!(g.delta > 0.4 && g.delta < 0.6);
    assert!((g.pos_delta + 65.0 * g.delta).abs() < 1e-9, "a SOLD option: position = -units x per-unit");
    let bad = crate::gk::bar_greeks(0, 2_500_000, 100, 86_400, true, 55_199, -65);
    assert!(!bad.ok && bad.refusal.contains("spot"), "a refusal is named");
}

/// ADDENDA 13 / 15: the screener's previous-close gain measure: the member's gain runs from the previous session's close, not the window
/// start (Rs 7.80 -> Rs 92.00 = +1,079.48%, PIIND's Dhan number; the window start Rs 19.00 would give +384.21%).
#[test]
fn v32_gain_measure_prev_close() {
    let mut c = cube(1, "PIIND", 2026_11_23, 175);
    c.close = vec![9_200; crate::cal::NM];
    c.cons[0].first_open = 1_900;
    c.cons[0].first_min = 0;
    c.cumvol = (0..=crate::cal::NM as u64).map(|m| m * 1_750).collect();
    c.cumval = (0..=crate::cal::NM as u64).map(|m| m * 1_750 * 9_200).collect();
    let w = crate::screen::member(&c, 0, 0, 60, true).unwrap();
    assert_eq!((w.c0, w.gain_bp), (1_900, (9_200 - 1_900) * 10_000 / 1_900), "the window measure: from today's first open");
    let p = crate::screen::member_prev(&c, 0, 0, 60, true, 780).unwrap();
    assert_eq!((p.c0, p.gain_bp), (780, 107_948), "+1,079.48% from the previous close Rs 7.80");
    assert!(crate::screen::member_prev(&c, 0, 0, 60, true, 0).is_none(), "no previous close: no member");
}

/// CHECK M2 / S3: the X view is detailed and every pick's 10% twin is the same account at share level 0.
#[test]
fn v32_wanted_picks_x_and_twins() {
    reset();
    let sm = Sum { list: 7, x: 3 * 5 + 1, n: 2, ..Sum::default() };
    assert_eq!(crate::report3::twin10(&sm).map(|s| (s.list, s.x, s.n)), Some((7, 15, 2)), "the 25% pick's twin: same list, setting, N at 10%");
    assert!(crate::report3::twin10(&Sum { x: 15, ..sm }).is_none(), "a 10% pick has no twin");
    assert!(!crate::report3::wanted_picks(9).iter().any(|p| p.0 == "X-SEARCH"), "v3.1: no X detail");
    sw::set(sw::PICKS32, true);
    sw::set(sw::SEARCH15, true);
    let w = crate::report3::wanted_picks(9);
    for name in ["HEADLINE-X", "X-SEARCH", "HEADLINE-V", "V-SEARCH"] {
        assert!(w.iter().any(|p| p.0 == name), "{name} detailed");
    }
    reset();
}

/// ADDENDUM 11 through the host: under margin rule v3 an out-of-the-money STOCK option's exposure is the AEL OTH row (3.5%), not the OTM
/// row (5.25%) rule v2 read.
#[test]
fn v32_host_margin_v3_stock() {
    reset();
    let d = day();
    let mut t = String::from("date\tund\tseg\texpiry\tright\tstrike\tfile_price\tund_file_price\tspan_unit_short\tscan_unit\tworst_scenario\tsom_unit\telm_rate_pct\telm_class\telm_oth_pct\telm_otm_pct\tlot\tdelta\tseries_price_scan\tfile\tfile_created\tael_file\n");
    t.push_str("2026-11-04\tSBIN\tstk\t20261111\tC\t1000\t10\t900\t100\t100\t11\t0\t5.25\tOTM\t3.5\t5.25\t\t0.5\t142\tnsccl.20261104.i1\tx\tael\n");
    let s = Snapshot::parse(t.as_bytes()).unwrap();
    let dm = DayM { day: d, fallback_from: None, snaps: vec![(i64::MIN, "nsccl.20261104.i1".to_owned(), s)] };
    let c = cube(1, "SBIN", 2026_11_11, 50);
    let mut lat = crate::mtable::Lat::default();
    let cm: ConM = crate::mtable::resolve(&dm, &c, &mut lat);
    sw::set(sw::MARGIN_V2, true);
    // the CALL at 1,000 with the stock at 900: out of the money
    let v2 = exact_at(&dm, &cm, 0, at_sod(d, 36_000), 90_000, 50, ymd_of(d), None);
    assert_eq!(v2.parts.rate_h, 525, "rule v2: the OTM row");
    sw::set(sw::MARGIN_V3, true);
    let v3 = exact_at(&dm, &cm, 0, at_sod(d, 36_000), 90_000, 50, ymd_of(d), None);
    assert_eq!(v3.parts.rate_h, 350, "rule v3: the OTH row whatever the moneyness");
    assert_eq!(v2.lot_exact - v3.lot_exact, (525 - 350) * 90_000 * 50 / 10_000, "rule v2 overstated by (5.25% - 3.5%) x S x q");
    reset();
}
