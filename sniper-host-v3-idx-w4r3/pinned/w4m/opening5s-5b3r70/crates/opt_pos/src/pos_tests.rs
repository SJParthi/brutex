//! Matrix cases of plan s4.7 for the positional legs on synthetic books (red first: each K11 / B mutant of s4.7 (17) turns one
//! of these red; the real-week cases are G-ROLL, `g-roll` + the node walker).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic, clippy::expect_used, reason = "tests")]

use super::*;
use idx_cal::day::from_civil;
use std::fmt::Write as _;

fn d(m: u32, dd: u32) -> Day {
    from_civil(2025, m, dd)
}

fn sec(day: Day, h: i64, m: i64, s: i64, px: i64, vol: u64) -> Sec {
    Sec { ts: at_sod(day, h * 3600 + m * 60 + s), high: px, low: px, close: px, vol }
}

fn at(day: Day, h: i64, m: i64, s: i64) -> i64 {
    at_sod(day, h * 3600 + m * 60 + s)
}

const FREEZE: &str = "index\teffective_from\tfreeze_qty_units\tcircular_number\tcircular_date\nNIFTY\t2025-01-01\t1800\tA\t2024-12-30\n";
const FREEZE_CUT: &str = "index\teffective_from\tfreeze_qty_units\tcircular_number\tcircular_date\nNIFTY\t2025-01-01\t1800\tA\t2024-12-30\nNIFTY\t2025-08-28\t900\tB\t2025-08-26\n";

struct World {
    sessions: Vec<Day>,
    cal: BTreeMap<Day, CalRow>,
    series: Vec<SeriesRow>,
    ladder: String,
    freeze: &'static str,
}

/// D1 2025-08-25, D2 08-26, D3 08-28 (expiry E1), D4 08-29, D5 09-01, D6 09-02 (E2); files = the previous session.
fn base(ce_next: &str) -> World {
    let sessions = vec![d(8, 25), d(8, 26), d(8, 28), d(8, 29), d(9, 1), d(9, 2)];
    let files = [d(8, 22), d(8, 25), d(8, 26), d(8, 28), d(8, 29), d(9, 1)];
    let mut cal = BTreeMap::new();
    for (k, &s) in sessions.iter().enumerate() {
        let f = files[k];
        let (kl, sid, ne) = if s <= d(8, 28) { (d(8, 28), 1, d(9, 2)) } else { (d(9, 2), 2, d(9, 9)) };
        cal.insert(s, CalRow { file_read: f, known_listed: kl, known_eff: kl, series: sid, next_eff: Some(ne), d_exp: s == kl });
    }
    let series = vec![SeriesRow { id: 1, aliases: vec![(d(8, 28), d(8, 22), d(8, 28))], true_last: d(8, 28) }, SeriesRow { id: 2, aliases: vec![(d(9, 2), d(8, 22), d(9, 2))], true_last: d(9, 2) }];
    let mut ladder = String::from("file_date\tform\tindex\texpiry\tce_strikes\tpe_strikes\tlots\toi_gcd\toi_rows\tfile_cands\n");
    for f in files {
        for e in [d(8, 28), d(9, 2), d(9, 9)] {
            let ce = if e == d(8, 28) { "2400000" } else { ce_next };
            ladder.push_str(&format!("{}\tudiff\tNIFTY\t{}\t{ce}\t2600000\t75\t-\t-\t-\n", day::fmt(f), day::fmt(e)));
        }
    }
    World { sessions, cal, series, ladder, freeze: FREEZE }
}

fn book(w: &World) -> Book {
    let mut rows = String::from("day\tfno_end\tindex_close\tsource\n");
    for s in &w.sessions {
        rows.push_str(&format!("{}\t15:30:00\t15:30:00\tregular\n", day::fmt(*s)));
    }
    let sess = Sessions::parse(&rows, "day\tsegment\tfrom\tresume\tsource\n").unwrap();
    let mut close = BTreeMap::new();
    for s in &w.sessions {
        close.insert(*s, 2_450_000);
    }
    let b = Book::new("NIFTY", w.cal.clone(), w.series.clone(), Ladder::parse(&w.ladder).unwrap(), Folders::default(), FreezeTable::parse(w.freeze).unwrap(), close, sess, None);
    for s in &w.sessions {
        b.put_spot(*s, vec![sec(*s, 9, 30, 0, 2_450_000, 0), sec(*s, 14, 59, 59, 2_450_000, 0)]);
    }
    b
}

/// The E1 contract on D1 (entry) and D3 (roll close), the E2 contract on D3 (roll open) and D4 (exit): `vol` a second.
fn fill_basic(b: &Book, vol: u64) {
    let e1 = "2025-08-28-2400000-CE";
    let e2 = "2025-09-02-2400000-CE";
    let mut r1: Vec<Sec> = vec![sec(d(8, 25), 9, 30, 0, 50_000, 75)];
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 50_000, vol)));
    b.put_opt(e1, d(8, 25), Some(r1));
    let mut r3: Vec<Sec> = vec![sec(d(8, 28), 9, 30, 0, 52_000, 75), sec(d(8, 28), 14, 59, 59, 52_000, 75)];
    r3.extend((0..60).map(|s| sec(d(8, 28), 15, 0, s, 53_000, vol)));
    b.put_opt(e1, d(8, 28), Some(r3));
    let mut n3: Vec<Sec> = vec![sec(d(8, 28), 14, 0, 0, 60_000, 75)];
    n3.extend((1..60).map(|s| sec(d(8, 28), 15, 0, s, 60_000, vol)));
    b.put_opt(e2, d(8, 28), Some(n3));
    let mut n4: Vec<Sec> = vec![sec(d(8, 29), 9, 30, 0, 61_000, 75)];
    n4.extend((0..60).map(|s| sec(d(8, 29), 10, 0, s, 61_000, vol)));
    b.put_opt(e2, d(8, 29), Some(n4));
}

fn spec(slot: i64, bp: u16, rule: OpenRule) -> Spec {
    Spec { streams: vec![Stream { book: 0, ce: true, entry: at(d(8, 25), 10, 0, 0), exit: at(d(8, 29), 10, 0, 0), slot, label: "NIFTY CE".into() }], account: 200_000_000, bp, kept: false, open_rule: rule, target: 50_000_000, slot_rule: SlotRule::default() }
}

fn kinds(o: &StreamOut) -> Vec<&'static str> {
    o.legs.iter().map(|l| l.kind).collect()
}

/// s4.2 (1)-(2): the roll on the TRUE expiry day at 15:00:00, the open at t2 = the END of the close's last filling second, the
/// next expiry's contract, then the forced exit from the NEW contract (s4.7 (7) last bullet). m_idx_roll_late: a leg after the
/// held expiry fails the guard (s4.7 (16)).
#[test]
fn s47_roll_at_1500_open_at_t2_exit_from_new_contract() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close", "roll_open", "exit"]);
    assert_eq!(o.legs[0].want, 975, "C11 closest of Rs 5 L at Rs 500 with lot 75: 13 lots");
    assert_eq!(o.legs[1].decided, at(d(8, 28), 15, 0, 0));
    assert_eq!(o.legs[1].known_at, at(d(8, 28), 15, 0, 1));
    assert_eq!(o.legs[2].decided, at(d(8, 28), 15, 0, 1), "sequential: t2 = the END of the close's last filling second");
    assert_eq!(o.legs[2].folder, "2025-09-02-2400000-CE");
    assert_eq!(o.legs[3].folder, "2025-09-02-2400000-CE");
    assert_eq!(o.trips.len(), 2);
    assert_eq!(o.flags.get("rolls"), Some(&1));
}

/// B2 (m_idx_roll_funds): a close that needs three seconds (share 10%) opens at the END of its THIRD second, never earlier.
#[test]
fn b2_sequential_open_waits_for_the_last_close_unit() {
    let b = book(&base("2400000"));
    fill_basic(&b, 4_000);
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 1_000, OpenRule::Sequential)).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close", "roll_open", "exit"]);
    assert_eq!(o.legs[1].fills.len(), 3, "375 + 375 + 225 units");
    assert_eq!(o.legs[2].decided, at(d(8, 28), 15, 0, 3), "open at t2 = 15:00:03");
}

/// B1 with R2 (m_idx_close_one_order): 975 units opened at freeze 1,800 close on a 900 day: 2 orders at once, one quantity.
#[test]
fn b1_close_split_after_a_freeze_cut() {
    let mut w = base("2400000");
    w.freeze = FREEZE_CUT;
    let b = book(&w);
    fill_basic(&b, 100_000);
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    let o = &r.streams[0];
    assert_eq!(o.legs[1].max_order, 900);
    assert_eq!(o.legs[1].slices, 2, "close_split 2 (R2: whole-lot orders at or below 900)");
    assert_eq!(o.legs[1].fills.len(), 1, "the slices share each block: one quantity, one fill row");
    assert_eq!(o.flags.get("close_split"), Some(&1));
    assert!(o.legs[2].want <= 900, "the open leg is fitted at the new maximum");
}

/// B3 (m_idx_redate): a series listed as 08-29 when opened and re-dated to 08-28 rolls on 08-28, never on its stale date.
#[test]
fn b3_redated_series_rolls_on_its_true_day() {
    let mut w = base("2400000");
    w.series[0] = SeriesRow { id: 1, aliases: vec![(d(8, 29), d(8, 22), d(8, 25)), (d(8, 28), d(8, 26), d(8, 28))], true_last: d(8, 28) };
    w.cal.insert(d(8, 25), CalRow { file_read: d(8, 22), known_listed: d(8, 29), known_eff: d(8, 29), series: 1, next_eff: Some(d(9, 2)), d_exp: false });
    w.ladder.push_str("2025-08-22\tudiff\tNIFTY\t2025-08-29\t2400000\t2600000\t75\t-\t-\t-\n");
    let b = book(&w);
    fill_basic(&b, 100_000);
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    assert_eq!(r.streams[0].legs[1].day, d(8, 28), "the roll on the true expiry day");
}

/// B3 (m_idx_folder): a series listed as 08-28 (a holiday here) expires on 08-26 by the holiday rule; its folder is named
/// 08-28. The roll reads DP-IDXCAL, never the folder's name.
#[test]
fn b3_folder_name_is_never_the_expiry() {
    let mut w = base("2400000");
    w.sessions.retain(|&s| s != d(8, 28));
    w.cal.remove(&d(8, 28));
    w.cal.insert(d(8, 26), CalRow { file_read: d(8, 25), known_listed: d(8, 28), known_eff: d(8, 26), series: 1, next_eff: Some(d(9, 2)), d_exp: true });
    w.cal.insert(d(8, 29), CalRow { file_read: d(8, 26), known_listed: d(9, 2), known_eff: d(9, 2), series: 2, next_eff: Some(d(9, 9)), d_exp: false });
    w.series[0] = SeriesRow { id: 1, aliases: vec![(d(8, 28), d(8, 22), d(8, 26))], true_last: d(8, 26) };
    let b = book(&w);
    let e1 = "2025-08-28-2400000-CE";
    let e2 = "2025-09-02-2400000-CE";
    let mut r1 = vec![sec(d(8, 25), 9, 30, 0, 50_000, 75)];
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 50_000, 100_000)));
    b.put_opt(e1, d(8, 25), Some(r1));
    let mut r2 = vec![sec(d(8, 26), 14, 59, 59, 52_000, 75)];
    r2.extend((0..60).map(|s| sec(d(8, 26), 15, 0, s, 53_000, 100_000)));
    b.put_opt(e1, d(8, 26), Some(r2));
    let mut n2 = vec![sec(d(8, 26), 14, 0, 0, 60_000, 75)];
    n2.extend((1..60).map(|s| sec(d(8, 26), 15, 0, s, 60_000, 100_000)));
    b.put_opt(e2, d(8, 26), Some(n2));
    let mut n4 = vec![sec(d(8, 29), 9, 30, 0, 61_000, 75)];
    n4.extend((0..60).map(|s| sec(d(8, 29), 10, 0, s, 61_000, 100_000)));
    b.put_opt(e2, d(8, 29), Some(n4));
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    assert_eq!(r.streams[0].legs[1].kind, "roll_close");
    assert_eq!(r.streams[0].legs[1].day, d(8, 26), "the holiday rule's day, read from DP-IDXCAL");
}

/// Leftovers at the expiry day's F&O end (s4.2 (3), B6): settled when the settlement is the worse reading (m_idx_settle), and
/// bullet 1 read from W0 = the 15:00:00 roll close, never from the E0 of an earlier day (m_idx_settle_window); no open leg after
/// an incomplete close (s4.7 (26)).
#[test]
fn b6_leftover_settles_and_the_window_starts_at_w0() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let e1 = "2025-08-28-2400000-CE";
    let mut r3 = vec![sec(d(8, 28), 10, 0, 0, 1_000, 75), sec(d(8, 28), 14, 59, 59, 52_000, 75)];
    r3.extend((0..5).map(|s| Sec { ts: at(d(8, 28), 15, 0, s), high: 53_000, low: 52_500, close: 53_000, vol: 750 }));
    b.put_opt(e1, d(8, 28), Some(r3));
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 1_000, OpenRule::Sequential)).unwrap();
    let o = &r.streams[0];
    // the entry at 10% of 100,000 a second fills 975 at once; the close takes 75 a second for 5 seconds: 600 left
    assert_eq!(o.trips[0].leftover, 600);
    assert_eq!(o.trips[0].bullet1_px, 52_500, "bullet 1 from W0 = 15:00:00 (the 10:00 print of 10.00 is before W0)");
    assert_eq!(o.trips[0].intrinsic, 50_000, "CE 24,000 against the 24,500 close");
    assert!(o.trips[0].settled, "the settlement (Rs 500 + exercise STT) is the worse reading");
    assert_eq!(o.trips[0].net, o.trips[0].net_settle);
    assert!(o.trips[0].net_settle < o.trips[0].net_bullet1);
    assert_eq!(o.flags.get("roll_close_incomplete"), Some(&1));
    assert_eq!(kinds(o), vec!["entry", "roll_close"]);
}

/// m_idx_roll_lead: the next expiry's nearest ITM strike whose first print starts AT t2 has no trade before t2: skipped, the
/// deeper strike is picked (s4.2 (2), s4.7 (10)).
#[test]
fn k11_roll_open_reads_prints_strictly_before_t2() {
    let b = book(&base("2350000,2400000"));
    fill_basic(&b, 100_000);
    let mut a = vec![sec(d(8, 28), 15, 0, 1, 60_000, 100_000)];
    a.extend((2..60).map(|s| sec(d(8, 28), 15, 0, s, 60_000, 100_000)));
    b.put_opt("2025-09-02-2400000-CE", d(8, 28), Some(a));
    let mut c = vec![sec(d(8, 28), 14, 0, 0, 110_000, 75)];
    c.extend((1..60).map(|s| sec(d(8, 28), 15, 0, s, 110_000, 100_000)));
    b.put_opt("2025-09-02-2350000-CE", d(8, 28), Some(c));
    b.put_opt("2025-09-02-2350000-CE", d(8, 29), Some((0..60).map(|s| sec(d(8, 29), 10, 0, s, 111_000, 100_000)).collect()));
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    let o = &r.streams[0];
    assert_eq!(o.legs[2].kind, "roll_open");
    assert_eq!(o.legs[2].strike, 2_350_000, "the 24,000 strike first printed AT t2: no trade before t2");
    assert_eq!(o.legs[2].pick.as_ref().map(|p| p.skipped_no_trade), Some(1));
}

/// m-IX14 / m_idx_spot: moneyness from the last index second STRICTLY before the decision (the 10:00:00 second has not ended).
#[test]
fn k11_spot_strictly_before_the_decision() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    b.put_spot(d(8, 25), vec![sec(d(8, 25), 9, 59, 59, 2_450_000, 0), sec(d(8, 25), 10, 0, 0, 2_390_000, 0)]);
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    assert_eq!(r.streams[0].legs[0].kind, "entry", "24,000 CE is ITM against 24,500 (09:59:59), not against 23,900 (10:00:00)");
    assert_eq!(r.streams[0].legs[0].pick.as_ref().map(|p| p.spot), Some(2_450_000));
}

/// S7 BUILDER wave 4 (lane B's P3 set on the index legs): the ENTRY pick reads only prints STRICTLY before the entry decision: the
/// nearest ITM strike (24,000) whose first print of the day is AT 10:00:00 has no trade before t and is skipped (counted); the deeper
/// 23,500 is picked. Red under `m_pos_leader` (prints at or after t read: lane B's full-day leader mapped to the A2 ITM fit).
#[test]
fn k11_entry_pick_reads_prints_strictly_before_t() {
    let mut w = base("2400000");
    w.ladder = w.ladder.replace("\t2025-08-28\t2400000\t", "\t2025-08-28\t2350000,2400000\t");
    let b = book(&w);
    let mut near: Vec<Sec> = (0..60).map(|s| sec(d(8, 25), 10, 0, s, 50_000, 100_000)).collect();
    near.extend((0..60).map(|s| sec(d(8, 25), 10, 30, s, 50_000, 100_000)));
    b.put_opt("2025-08-28-2400000-CE", d(8, 25), Some(near));
    let mut deep: Vec<Sec> = vec![sec(d(8, 25), 9, 30, 0, 100_000, 75)];
    deep.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 100_000, 100_000)));
    deep.extend((0..60).map(|s| sec(d(8, 25), 10, 30, s, 101_000, 100_000)));
    b.put_opt("2025-08-28-2350000-CE", d(8, 25), Some(deep));
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 25), 10, 30, 0);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "exit"]);
    assert_eq!(o.legs[0].strike, 2_350_000, "the 24,000 strike first printed AT the decision: no trade before t");
    assert_eq!(o.legs[0].pick.as_ref().map(|p| p.skipped_no_trade), Some(1));
    assert_eq!(o.legs[0].pick.as_ref().map(|p| p.reference), Some(100_000), "the size reference is the deeper strike's 09:30 print");
}

/// S7 BUILDER wave 4 (lane B's P3 (d) on the index legs): the protection limit of every order reads the contract's last print STRICTLY
/// before its anchor (the entry's 09:30 print of Rs 500, not the Rs 520 print AT 10:00:00; the exit's 10:29:59 print of Rs 550, not the
/// Rs 530 print at 10:30:00). Red under `m_pos_ref` (the first print at or after the anchor read).
#[test]
fn k11_protection_reference_is_the_last_print_before_the_anchor() {
    let b = book(&base("2400000"));
    let mut r1: Vec<Sec> = vec![sec(d(8, 25), 9, 30, 0, 50_000, 75)];
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 52_000, 100_000)));
    r1.push(sec(d(8, 25), 10, 29, 59, 55_000, 75));
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 30, s, 53_000, 100_000)));
    b.put_opt("2025-08-28-2400000-CE", d(8, 25), Some(r1));
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.kept = true;
    sp.streams[0].exit = at(d(8, 25), 10, 30, 0);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "exit"]);
    assert_eq!(o.legs[0].limit, Some(limits::protection(50_000, Side::Buy, 1_000, 5)), "buy limit from the 09:30 print");
    assert_ne!(o.legs[0].limit, Some(limits::protection(52_000, Side::Buy, 1_000, 5)));
    assert_ne!(o.legs[1].limit, Some(limits::protection(53_000, Side::Sell, 1_000, 5)));
    assert_eq!(o.legs[1].limit, Some(limits::protection(55_000, Side::Sell, 1_000, 5)), "sell limit from the 10:29:59 print");
}

/// B2 (s4.7 (20)): at N = 1 the sequential roll rolls with the close proceeds; the same-decision reading is refused for cash.
#[test]
fn b2_n1_roll_sequential_rolls_same_decision_refused() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let a = run(std::slice::from_ref(&b), &spec(200_000_000, 10_000, OpenRule::Sequential)).unwrap();
    assert_eq!(a.streams[0].legs[0].want, 3_975, "Rs 20 L at Rs 500 x 75: 53 lots");
    assert_eq!(a.streams[0].legs[0].slices, 3, "above one order: sliced beside (R5)");
    assert_eq!(a.streams[0].flags.get("rolls"), Some(&1));
    let bb = run(std::slice::from_ref(&b), &spec(200_000_000, 10_000, OpenRule::SameDecision)).unwrap();
    assert_eq!(bb.streams[0].flags.get("roll_refused_cash"), Some(&1), "(b): the close's money is not yet released at 15:00:00");
}

// ---------- s8b (S8 BUILDER, wave 4): check-build-idx-S7-S10 D-S8-1 / D-S8-2 and MAIN-RULINGS-S7-S10 item 2 ----------

/// F4 bullet 1 (plan s4.2 (4), s4.7 (7)): an exit decided at exactly 15:00:00 on the roll day is exit only: no roll close, no re-open;
/// the expiring contract is sold from 15:00:00.
#[test]
fn f4_exit_at_1500_on_the_roll_day_is_exit_only() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 28), 15, 0, 0);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "exit"]);
    assert_eq!(o.legs[1].decided, at(d(8, 28), 15, 0, 0));
    assert_eq!(o.legs[1].folder, "2025-08-28-2400000-CE");
    assert_eq!(o.legs[1].units, 975);
    assert_eq!(o.flags.get("rolls"), None);
    assert_eq!(o.flags.get("roll_cancelled_by_flip"), None);
}

/// F4 bullet 2 (plan s4.2 (4), s4.7 (7)): a flip at 15:00:05 while the roll close is still filling (75 a second: t2 = 15:00:13): the
/// position is already closing, so no open leg is sent (`roll_cancelled_by_flip`); the close keeps filling to its last unit.
/// m_idx_flip_open: the open leg is still decided at t2 and never closed.
#[test]
fn f4_flip_while_the_close_fills_sends_no_open_leg() {
    let b = book(&base("2400000"));
    fill_basic(&b, 750);
    let mut sp = spec(50_000_000, 1_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 28), 15, 0, 5);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close"], "no open leg after the flip");
    assert_eq!(o.legs[1].units, 975, "the close keeps filling to its last unit");
    assert_eq!(o.legs[1].known_at, at(d(8, 28), 15, 0, 13), "975 units at 75 a second: t2 = 15:00:13");
    assert_eq!(o.flags.get("roll_cancelled_by_flip"), Some(&1));
    assert_eq!(o.flags.get("open_after_run"), None);
    assert_eq!(o.trips.len(), 1);
}

/// F4 bullet 3 (plan s4.2 (4), s4.7 (7)): a flip at 15:00:05 while the open leg (from t2 = 15:00:01, 75 a second) is part-filled: the
/// 300 units known by the flip exit from the next block, and the 525 not filled are cancelled and counted. m_idx_exit_cut: the open
/// walks on to 825 units and the exit sells units bought after the flip.
#[test]
fn f4_flip_while_the_open_fills_exits_the_filled_units() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let e2 = "2025-09-02-2400000-CE";
    let mut n3: Vec<Sec> = vec![sec(d(8, 28), 14, 0, 0, 60_000, 75)];
    n3.extend((1..60).map(|s| sec(d(8, 28), 15, 0, s, 60_000, 750)));
    b.put_opt(e2, d(8, 28), Some(n3));
    let mut sp = spec(50_000_000, 1_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 28), 15, 0, 5);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close", "roll_open", "exit"]);
    assert_eq!(o.legs[2].decided, at(d(8, 28), 15, 0, 1));
    assert_eq!(o.legs[2].want, 825, "C11 closest of Rs 5 L at Rs 600 x 75: 11 lots");
    assert_eq!(o.legs[2].units, 300, "only the units known by the flip (blocks 15:00:01-15:00:04) are held");
    assert_eq!(o.legs[3].want, 300);
    assert_eq!(o.legs[3].folder, e2);
    assert_eq!(o.legs[3].decided, at(d(8, 28), 15, 0, 5));
    assert_eq!(o.flags.get("open_cut_by_flip"), Some(&1));
    assert_eq!(o.flags.get("open_cut_units"), Some(&525));
    assert_eq!(o.flags.get("roll_short"), None);
}

/// F4 bullet 4 (plan s4.2 (4), s4.7 (7)): a flip at 15:00:30, after the open leg completed in the block 15:00:01: the exit sells the
/// NEW contract the same session.
#[test]
fn f4_flip_after_the_open_completed_exits_the_new_contract() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 28), 15, 0, 30);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close", "roll_open", "exit"]);
    assert_eq!(o.legs[2].units, o.legs[2].want, "the open completed before the flip");
    assert_eq!(o.legs[3].folder, "2025-09-02-2400000-CE");
    assert_eq!(o.legs[3].day, d(8, 28));
    assert_eq!(o.legs[3].want, o.legs[2].units);
    assert_eq!(o.flags.get("open_cut_by_flip"), None);
    assert_eq!(o.flags.get("roll_cancelled_by_flip"), None);
}

/// D-S8-1 X3 (an exit decided while the ENTRY is still filling): at share 10% the 975-unit entry fills 75 a second from 10:00:00; an
/// exit decided at 10:00:03 holds only the 225 units known by then and sells them from 10:00:03; the 750 not filled are cancelled and
/// counted. m_idx_exit_cut: all 975 are booked and sold, units bought after the exit included.
#[test]
fn d_s8_1_exit_during_the_entry_fill_holds_only_known_units() {
    let b = book(&base("2400000"));
    fill_basic(&b, 750);
    let mut sp = spec(50_000_000, 1_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 25), 10, 0, 3);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "exit"]);
    assert_eq!(o.legs[0].want, 975);
    assert_eq!(o.legs[0].units, 225, "blocks 10:00:00-10:00:02, known by 10:00:03");
    assert_eq!(o.legs[0].known_at, at(d(8, 25), 10, 0, 3));
    assert_eq!(o.legs[1].want, 225);
    assert_eq!(o.legs[1].fills.first().map(|f| f.0), Some(at(d(8, 25), 10, 0, 3)));
    assert_eq!(o.trips[0].units_in, 225);
    assert_eq!(o.flags.get("entry_cut_by_exit"), Some(&1));
    assert_eq!(o.flags.get("entry_cut_units"), Some(&750));
}

/// s4.2 (5) bullet 3 / s4.7 (8): an entry at 14:59:50 on its expiry day takes the expiring contract; at share 10% it fills 75 a
/// second, so 750 units are known at 15:00:00; the 225 not filled are cancelled at the roll (counted) and the 750 filled units roll.
#[test]
fn s42_5_entry_rest_cancelled_at_the_roll_and_counted() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let e1 = "2025-08-28-2400000-CE";
    let mut r3: Vec<Sec> = vec![sec(d(8, 28), 9, 30, 0, 52_000, 75)];
    r3.extend((50..60).map(|s| sec(d(8, 28), 14, 59, s, 52_000, 750)));
    r3.extend((0..60).map(|s| sec(d(8, 28), 15, 0, s, 53_000, 100_000)));
    b.put_opt(e1, d(8, 28), Some(r3));
    let mut sp = spec(50_000_000, 1_000, OpenRule::Sequential);
    sp.streams[0].entry = at(d(8, 28), 14, 59, 50);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close", "roll_open", "exit"]);
    assert_eq!(o.legs[0].folder, e1, "before 15:00:00: the expiring contract (Q24)");
    assert_eq!(o.legs[0].want, 975, "C11 closest of Rs 5 L at Rs 520 x 75: 13 lots");
    assert_eq!(o.legs[0].units, 750);
    assert_eq!(o.legs[1].want, 750, "the filled units roll");
    assert_eq!(o.flags.get("entry_rest_cancelled_at_roll"), Some(&1));
    assert_eq!(o.flags.get("entry_rest_cancelled_at_roll_units"), Some(&225));
}

/// D-S8-2 (plan s4.2 (3), B6): a position entered on its expiry day at 10:00:00 (Q24) whose 15:00:00 roll close leaves leftovers: S15
/// bullet 1's window starts at W0 = E0 (10:00:00), so the 11:00:00 print at Rs 10 is inside it and bullet 1 is the worse reading.
/// m_idx_w0_same_session: W0 = 15:00:00 and the leftovers settle at intrinsic.
#[test]
fn d_s8_2_same_session_entry_window_starts_at_e0() {
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    let e1 = "2025-08-28-2400000-CE";
    let mut r3: Vec<Sec> = vec![sec(d(8, 28), 9, 30, 0, 52_000, 75)];
    r3.extend((0..60).map(|s| sec(d(8, 28), 10, 0, s, 52_000, 100_000)));
    r3.push(Sec { ts: at(d(8, 28), 11, 0, 0), high: 52_000, low: 1_000, close: 52_000, vol: 75 });
    r3.push(sec(d(8, 28), 14, 59, 59, 52_000, 75));
    r3.extend((0..5).map(|s| Sec { ts: at(d(8, 28), 15, 0, s), high: 53_000, low: 52_500, close: 53_000, vol: 750 }));
    b.put_opt(e1, d(8, 28), Some(r3));
    let mut sp = spec(50_000_000, 1_000, OpenRule::Sequential);
    sp.streams[0].entry = at(d(8, 28), 10, 0, 0);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close"]);
    assert_eq!(o.legs[0].e0, at(d(8, 28), 10, 0, 0));
    assert_eq!(o.trips[0].leftover, 600, "the close sells 75 a second for 5 seconds: 375 of 975");
    assert_eq!(o.trips[0].w0, at(d(8, 28), 10, 0, 0), "W0 = E0: entered that same session");
    assert_eq!(o.trips[0].bullet1_px, 1_000, "the 11:00:00 print is inside [W0, F&O end)");
    assert!(!o.trips[0].settled, "bullet 1 (Rs 10) is the worse reading");
    assert_eq!(o.trips[0].net, o.trips[0].net_bullet1);
    assert_eq!(o.flags.get("roll_close_incomplete"), Some(&1));
}

/// The two-stream world of the MIXED cases: CE and PE in ONE Rs 20 L account at N 2, entered 2025-08-25 10:00:00 at Rs 400; at the
/// shared 15:00:00 roll of 2025-08-28 the CALL sells at 5 paisa in one second; the PUT sells at Rs 800, 225 units a second for
/// `pe_secs` seconds.
fn mixed_world(pe_secs: i64) -> (Book, Spec) {
    let b = book(&base("2400000"));
    let (c1, p1, c2, p2) = ("2025-08-28-2400000-CE", "2025-08-28-2600000-PE", "2025-09-02-2400000-CE", "2025-09-02-2600000-PE");
    for f in [c1, p1] {
        let mut r: Vec<Sec> = vec![sec(d(8, 25), 9, 30, 0, 40_000, 75)];
        r.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 40_000, 1_000_000)));
        b.put_opt(f, d(8, 25), Some(r));
    }
    b.put_opt(c1, d(8, 28), Some((0..60).map(|s| sec(d(8, 28), 15, 0, s, 5, 1_000_000)).collect()));
    b.put_opt(p1, d(8, 28), Some((0..pe_secs).map(|s| sec(d(8, 28), 15, 0, s, 80_000, 250)).collect()));
    for f in [c2, p2] {
        let mut r: Vec<Sec> = vec![sec(d(8, 28), 14, 0, 0, 40_000, 75)];
        r.extend((1..60).map(|s| sec(d(8, 28), 15, 0, s, 40_000, 1_000_000)));
        b.put_opt(f, d(8, 28), Some(r));
        b.put_opt(f, d(8, 29), Some((0..60).map(|s| sec(d(8, 29), 10, 0, s, 41_000, 1_000_000)).collect()));
    }
    let mk = |ce: bool, label: &str| Stream { book: 0, ce, entry: at(d(8, 25), 10, 0, 0), exit: at(d(8, 29), 10, 0, 0), slot: 100_000_000, label: label.into() };
    let sp = Spec { streams: vec![mk(true, "NIFTY CE"), mk(false, "NIFTY PE")], account: 200_000_000, bp: 10_000, kept: false, open_rule: OpenRule::Sequential, target: 50_000_000, slot_rule: SlotRule::default() };
    (b, sp)
}

/// MAIN ruling 2 (MIXED rolls, plan s4.5 amended): the CALL's close loses almost its whole Rs 10 L and is known at 15:00:01; the PUT's
/// close gains it back and is known at 15:00:11. Every close of the instant is settled first: both re-opens are decided at 15:00:11
/// against base Rs 20 L + min(0, net so far), and both are funded. m_idx_roll_group: the CALL re-open is decided at 15:00:01, while
/// the PUT still holds Rs 9.9 L, and is refused for cash (the 2025-09-30 order dependence).
#[test]
fn mixed_closes_of_one_instant_settle_before_any_reopen() {
    let (b, sp) = mixed_world(60);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let (ce, pe) = (&r.streams[0], &r.streams[1]);
    assert_eq!(ce.legs[0].want, 2_475, "C11 closest of Rs 10 L at Rs 400 x 75: 33 lots");
    assert_eq!(pe.legs[0].want, 2_475);
    assert_eq!(ce.legs[1].known_at, at(d(8, 28), 15, 0, 1));
    assert_eq!(pe.legs[1].known_at, at(d(8, 28), 15, 0, 11), "2,475 units at 225 a second");
    assert_eq!(ce.legs[2].kind, "roll_open", "funded once every close of the instant is settled");
    assert_eq!(ce.legs[2].decided, at(d(8, 28), 15, 0, 11), "decided at the LAST close's t2");
    assert_eq!(ce.legs[2].want, 2_475);
    assert_eq!(pe.legs[2].kind, "roll_open");
    assert_eq!(pe.legs[2].decided, at(d(8, 28), 15, 0, 11));
    assert_eq!(ce.flags.get("roll_refused_cash"), None);
    assert_eq!(kinds(ce), vec!["entry", "roll_close", "roll_open", "exit"]);
    assert_eq!(kinds(pe), vec!["entry", "roll_close", "roll_open", "exit"]);
}

/// MAIN ruling 2 under (a): the PUT's close is still incomplete at the index close (5 seconds of prints: 1,125 of 2,475 sold), so it is
/// not settled before 15:30:00 and NO re-open of that instant can be decided: the PUT counts `roll_close_incomplete`, the CALL
/// `roll_wait_close_incomplete`. m_idx_roll_group: the CALL re-opens (refused for cash) at its own t2.
#[test]
fn mixed_reopen_waits_for_every_close_of_the_instant() {
    let (b, sp) = mixed_world(5);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let (ce, pe) = (&r.streams[0], &r.streams[1]);
    assert_eq!(pe.trips[0].leftover, 1_350);
    assert_eq!(pe.flags.get("roll_close_incomplete"), Some(&1));
    assert_eq!(ce.flags.get("roll_wait_close_incomplete"), Some(&1));
    assert_eq!(kinds(ce), vec!["entry", "roll_close"]);
    assert_eq!(kinds(pe), vec!["entry", "roll_close"]);
}

/// W4 REPAIR (check-wave4-S9 D-W4-3; plan s5.2, MAIN-RULINGS-S7-S10 item 2 "each slot is base / N"): `SlotRule::BaseOverN` is THE
/// headline (`SlotRule::default()`): after the CALL's close loses about Rs 4.87 L at N 4, the re-open is sized to (Rs 20 L + the net) /
/// 4 = 8 lots at Rs 600 x 75; the BESIDE fixed slot (Rs 5 L) sizes 11 lots against the same free cash.
#[test]
fn slot_rule_base_over_n_is_the_headline_fixed_beside() {
    assert_eq!(SlotRule::default(), SlotRule::BaseOverN, "the headline is base / N");
    let b = book(&base("2400000"));
    fill_basic(&b, 100_000);
    b.put_opt("2025-08-28-2400000-CE", d(8, 28), Some((0..60).map(|s| sec(d(8, 28), 15, 0, s, 5, 100_000)).collect()));
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.slot_rule = SlotRule::default();
    let base_n = run(std::slice::from_ref(&b), &sp).unwrap();
    assert!(base_n.streams[0].trips[0].net < -48_000_000);
    assert_eq!(base_n.streams[0].legs[2].want, 600, "headline: (Rs 20 L + net) / 4 = about Rs 3.78 L: 8 lots");
    sp.slot_rule = SlotRule::Fixed;
    let fixed = run(std::slice::from_ref(&b), &sp).unwrap();
    assert_eq!(fixed.streams[0].legs[2].want, 825, "beside: the fixed Rs 5 L slot");
}

/// W4 REPAIR (D-W4-3): ONE formula for the one account (`opt_pos::pos` and `opt_pos::acct` both call `SlotRule::slot`): base / N with
/// base = account + min(0, the net known), never below 0; a profit never grows the slot; the fixed reading is account / N.
#[test]
fn w4r_one_slot_formula_for_pos_and_acct() {
    let r = SlotRule::default();
    assert_eq!(r.slot(200_000_000, 4, 0), 50_000_000);
    assert_eq!(r.slot(200_000_000, 4, 30_000_000), 50_000_000, "profits are never added to the base");
    assert_eq!(r.slot(200_000_000, 4, -48_700_000), 37_825_000, "Rs 20 L minus the net loss, over N");
    assert_eq!(r.slot(200_000_000, 4, -300_000_000), 0, "never below 0");
    assert_eq!(r.slot(200_000_000, 0, -1), 199_999_999, "N below 1 reads 1");
    assert_eq!(SlotRule::Fixed.slot(200_000_000, 4, -48_700_000), 50_000_000, "beside: the fixed slot ignores the net");
    assert!(SlotRule::default().name().starts_with("base / N"));
}

/// A book whose session facts are the given F&O-end rows (index close 15:30:00) plus a MUHURAT-LIST text (W4 REPAIR cases).
fn book_sess(w: &World, fno: &[(Day, &str)], muhurat: &str) -> Book {
    let mut rows = String::from("day\tfno_end\tindex_close\tsource\n");
    for &(sd, end) in fno {
        let _ = writeln!(rows, "{}\t{end}\t15:30:00\tregular", day::fmt(sd));
    }
    let sess = Sessions::parse(&rows, "day\tsegment\tfrom\tresume\tsource\n").unwrap().with_muhurat(muhurat).unwrap();
    let mut close = BTreeMap::new();
    for s in &w.sessions {
        close.insert(*s, 2_450_000);
    }
    let b = Book::new("NIFTY", w.cal.clone(), w.series.clone(), Ladder::parse(&w.ladder).unwrap(), Folders::default(), FreezeTable::parse(w.freeze).unwrap(), close, sess, None);
    for s in &w.sessions {
        b.put_spot(*s, vec![sec(*s, 9, 30, 0, 2_450_000, 0), sec(*s, 14, 59, 59, 2_450_000, 0)]);
    }
    b
}

/// RESUMED WAVE 4: the PRODUCTION source end to end through `Book::with_newpx` (DP-NEWPX day files + the DP-NEWPX-FAILS sidecar,
/// written by the pack's own encoder): a decoder-failure second in the entry's first block reads the WORSE of the two sources
/// (store HIGH 510.00 against the corrected 500.00, volume 500 against 100,000), the entry leg carries the HOLE flag, every other
/// leg (clean contract-days) does not, and the legs' fills equal the store-injected run everywhere else (m_newpx_kinder: red).
#[test]
fn w4_production_source_worse_of_two_and_hole_flag_end_to_end() {
    use crate::newpx::Fails;
    use crate::pack::{self, NewSec};
    let dir = std::env::temp_dir().join(format!("w4m-newpx-pos-test-{}", std::process::id()));
    let put = |day: Day, blocks: &[(&str, Vec<NewSec>)]| {
        let mut raw = Vec::new();
        for (c, s) in blocks {
            raw.extend_from_slice(&pack::enc_block(c, s));
        }
        let ds = day::fmt(day);
        let p = dir.join(&ds[0..7]).join(&ds[0..4]);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join(format!("{ds}.newpx.zst")), pack::zst(&raw)).unwrap();
    };
    let ns = |h: i32, m: i32, s: i32, px: i64, vol: u64| -> NewSec { (h * 3600 + m * 60 + s, px, px, px, px, vol) };
    let vol = 100_000u64;
    let e1 = "2025-08-28-2400000-CE";
    let e2 = "2025-09-02-2400000-CE";
    let mut r1 = vec![ns(9, 30, 0, 50_000, 75)];
    r1.extend((0..60).map(|s| ns(10, 0, s, 50_000, vol)));
    put(d(8, 25), &[(e1, r1)]);
    let mut r3 = vec![ns(9, 30, 0, 52_000, 75), ns(14, 59, 59, 52_000, 75)];
    r3.extend((0..60).map(|s| ns(15, 0, s, 53_000, vol)));
    let mut n3 = vec![ns(14, 0, 0, 60_000, 75)];
    n3.extend((1..60).map(|s| ns(15, 0, s, 60_000, vol)));
    put(d(8, 28), &[(e1, r3), (e2, n3)]);
    let mut n4 = vec![ns(9, 30, 0, 61_000, 75)];
    n4.extend((0..60).map(|s| ns(10, 0, s, 61_000, vol)));
    put(d(8, 29), &[(e2, n4)]);
    let mut fails = Fails::default();
    fails
        .add("contract\tday\tsod\tkind\ts_high\ts_low\ts_close\ts_vol\tt_high\tt_low\tt_close\tt_vol\n2025-08-28-2400000-CE\t2025-08-25\t10:00:00\tboth\t51000\t50000\t50000\t500\t50000\t50000\t50000\t100000\n")
        .unwrap();
    let b = book(&base("2400000")).with_newpx(NewpxSrc::new(dir, fails));
    let r = run(std::slice::from_ref(&b), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "roll_close", "roll_open", "exit"]);
    assert_eq!(o.legs[0].want, 975);
    assert_eq!((o.legs[0].fills[0].1, o.legs[0].fills[0].3), (450, 51_000), "the failing second: the smaller volume (500 -> 6 whole lots of 75) at the higher HIGH");
    assert_eq!((o.legs[0].fills[1].1, o.legs[0].fills[1].3), (525, 50_000), "the rest from the next clean second");
    assert!(o.legs[0].hole, "the entry spans the decoder-failure second");
    assert!(o.legs[1..].iter().all(|l| !l.hole), "clean contract-days carry no HOLE flag");
    let counts = b.newpx().map(|s| s.counts.borrow().clone()).unwrap_or_default();
    assert_eq!(counts.get("seconds"), Some(&1));
    assert!(counts.get("agree").is_some_and(|n| *n >= 3));
    // the same run on the store-injected rows (no failure) fills all 975 at 500.00 in the first second and has no hole
    let s = book(&base("2400000"));
    fill_basic(&s, vol);
    let rs = run(std::slice::from_ref(&s), &spec(50_000_000, 10_000, OpenRule::Sequential)).unwrap();
    assert_eq!((rs.streams[0].legs[0].fills[0].1, rs.streams[0].legs[0].fills[0].3), (975, 50_000));
    assert!(!rs.streams[0].legs[0].hole);
    assert_eq!(rs.streams[0].legs[3].fills, o.legs[3].fills, "a clean leg fills the same from either source");
}

/// S7 BUILDER wave 4 resumed again (plan v3.1 s4.3a B2 / s4.4 "Data end" / s4.7 (15); the NIFTY 60m LONG positional canary stopped on
/// "no next session for the carry"): the book's data ends on 08-26; a position bought on 08-25 whose exit (08-26 15:29:00) sells only
/// 375 of 975 units by the F&O end finds no next session to carry to, so the 600 left are `open_at_data_end`: kept, valued at the worst
/// print of that LAST session (the 10:00 print of Rs 400; W0 = the session's first block, the trip was bought the session before),
/// charged as one more sale order, never settled, never an error. Red under m_idx_data_end_error (the run stops) and
/// m_idx_data_end_kind (valued from the exit's decision: Rs 530).
#[test]
fn data_end_rest_is_valued_at_the_worst_print_of_the_last_session() {
    let mut w = base("2400000");
    w.sessions = vec![d(8, 25), d(8, 26)];
    let b = book(&w);
    let e1 = "2025-08-28-2400000-CE";
    let mut r1: Vec<Sec> = vec![sec(d(8, 25), 9, 30, 0, 50_000, 75)];
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 50_000, 100_000)));
    b.put_opt(e1, d(8, 25), Some(r1));
    let mut r2: Vec<Sec> = vec![sec(d(8, 26), 10, 0, 0, 40_000, 75)];
    r2.extend((0..5).map(|s| sec(d(8, 26), 15, 29, s, 53_000, 750)));
    b.put_opt(e1, d(8, 26), Some(r2));
    let mut sp = spec(50_000_000, 1_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 26), 15, 29, 0);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "exit"]);
    assert_eq!(o.legs[1].units, 375, "75 a second for 5 seconds");
    let tr = &o.trips[0];
    assert_eq!(tr.leftover, 600);
    assert!(tr.data_end && !tr.settled);
    assert_eq!(tr.w0, at(d(8, 26), 9, 15, 0), "W0 = the last session's first block (bought the session before)");
    assert_eq!(tr.bullet1_px, 40_000, "the worst print of the last session, not the exit window's Rs 530");
    assert_eq!(tr.closed_at, at(d(8, 26), 15, 30, 0), "known at the last session's F&O end");
    assert_eq!(tr.net, tr.net_bullet1);
    assert!(tr.net < 375 * 53_000 + 600 * 53_000 - tr.value_in, "worse than selling the rest at the exit price");
    assert_eq!(tr.net + tr.charges, 375 * 53_000 + 600 * 40_000 - tr.value_in, "gross = the sale + the rest at the worst print");
    assert_eq!(o.flags.get("open_at_data_end"), Some(&1));
    assert_eq!(o.flags.get("open_at_data_end_units"), Some(&600));
    assert_eq!(o.flags.get("carry"), None, "nothing carried past the data end");
    assert_eq!(o.flags.get("open_after_run"), None, "the position is valued, never left open");
    assert_eq!(r.net, tr.net);
}

/// The same data end for a trip bought ON the last session: W0 = E0 (10:00:00, the entry's first filling block), so the 09:45 print
/// whose LOW is Rs 10 (before the position existed) is not read; the worst print after E0 is the 12:00 print of Rs 450. Red under
/// m_idx_data_end_e0 (the session's first block: Rs 10), m_idx_data_end_kind (Rs 530) and m_idx_data_end_error.
#[test]
fn data_end_same_session_buy_is_valued_from_e0() {
    let mut w = base("2400000");
    w.sessions = vec![d(8, 25)];
    let b = book(&w);
    let e1 = "2025-08-28-2400000-CE";
    let mut r1: Vec<Sec> = vec![Sec { ts: at(d(8, 25), 9, 45, 0), high: 50_000, low: 1_000, close: 50_000, vol: 75 }];
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 50_000, 100_000)));
    r1.push(sec(d(8, 25), 12, 0, 0, 45_000, 75));
    r1.extend((0..5).map(|s| sec(d(8, 25), 15, 29, s, 53_000, 750)));
    b.put_opt(e1, d(8, 25), Some(r1));
    let mut sp = spec(50_000_000, 1_000, OpenRule::Sequential);
    sp.streams[0].exit = at(d(8, 25), 15, 29, 0);
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry", "exit"]);
    assert_eq!(o.legs[0].want, 975, "the reference is the 09:45 HIGH of Rs 500: 13 lots");
    assert_eq!(o.legs[0].e0, at(d(8, 25), 10, 0, 0));
    let tr = &o.trips[0];
    assert_eq!(tr.leftover, 600);
    assert!(tr.data_end);
    assert_eq!(tr.w0, at(d(8, 25), 10, 0, 0), "W0 = E0: the trip was bought on the last session");
    assert_eq!(tr.bullet1_px, 45_000, "the worst print in [E0, F&O end), never the 09:45 LOW before E0");
    assert_eq!(o.flags.get("open_at_data_end"), Some(&1));
}

/// W4 REPAIR (check-wave4-S9 D-W4-1; plan v3.1 s4.3a B2 / s4.4 "Data end" / s4.7 (15)): a stream with NO exit before the data end
/// (the `DATA_END` sentinel) bought 975 units on 08-25; the data ends on 08-26 (F&O end 15:40:00, the index close 15:30:00, as on
/// 2026-10-30). No exit leg is sent: every held unit is `open_at_data_end`, valued WHOLE at the lowest LOW of that last session from its
/// first block (Rs 420 at 11:00:00; the trip was bought the session before), charged as one sale order, known at the F&O end, never
/// `open_after_run`. The 15:30-15:40 prints (Rs 600) are never sold on. Red under `m_idx_data_end_standin` (the final4 stand-in exit at
/// the last index close sells every unit at Rs 600: an exit leg, no data-end leftover, a kinder net).
#[test]
fn data_end_open_position_valued_whole_at_the_last_session_worst_print() {
    let mut w = base("2400000");
    w.sessions = vec![d(8, 25), d(8, 26)];
    let b = book_sess(&w, &[(d(8, 25), "15:30:00"), (d(8, 26), "15:40:00")], "");
    let e1 = "2025-08-28-2400000-CE";
    let mut r1: Vec<Sec> = vec![sec(d(8, 25), 9, 30, 0, 50_000, 75)];
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 50_000, 100_000)));
    b.put_opt(e1, d(8, 25), Some(r1));
    let mut r2: Vec<Sec> = vec![sec(d(8, 26), 9, 15, 0, 52_000, 75), sec(d(8, 26), 11, 0, 0, 42_000, 75), sec(d(8, 26), 14, 0, 0, 55_000, 75)];
    r2.extend((0..600).map(|s| Sec { ts: at(d(8, 26), 15, 30, 0) + s * 1_000_000, high: 60_000, low: 60_000, close: 60_000, vol: 100_000 }));
    b.put_opt(e1, d(8, 26), Some(r2));
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.streams[0].exit = DATA_END;
    sp.slot_rule = SlotRule::default();
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry"], "no exit leg: the position is never sold before the data end");
    assert_eq!(o.legs[0].units, 975);
    let tr = &o.trips[0];
    assert!(tr.sells.is_empty());
    assert_eq!(tr.leftover, 975, "every held unit is open_at_data_end");
    assert!(tr.data_end && !tr.settled);
    assert_eq!(tr.w0, at(d(8, 26), 9, 15, 0), "W0 = the last session's first block (bought the session before)");
    assert_eq!(tr.bullet1_px, 42_000, "the worst print of the last session (11:00:00), never the 15:30-15:40 prints");
    assert_eq!(tr.closed_at, at(d(8, 26), 15, 40, 0), "known at the last session's F&O end");
    assert_eq!(tr.net, tr.net_bullet1);
    assert_eq!(tr.net + tr.charges, 975 * 42_000 - tr.value_in, "gross = every unit at the worst print");
    assert!(tr.charges > 0, "charged as one sale order (the dated estimator)");
    assert!(tr.net < 975 * 60_000 - tr.value_in - tr.charges, "worse than any stand-in sale on the 15:30-15:40 prints");
    assert_eq!(o.flags.get("open_at_data_end"), Some(&1));
    assert_eq!(o.flags.get("open_at_data_end_units"), Some(&975));
    assert_eq!(o.flags.get("open_after_run"), None, "valued, never left open");
    assert_eq!(o.flags.get("carry"), None);
    assert_eq!(r.net, tr.net);
}

/// W4 REPAIR (D-W4-1), the same data end for a trip bought ON the last session: W0 = E0 (10:00:00), so the 09:45 print whose LOW is
/// Rs 10 (before the position existed) is not read; the worst print after E0 is the 12:00 print of Rs 450. Red under
/// `m_idx_data_end_e0` (the session's first block: Rs 10) and `m_idx_data_end_standin` (an exit leg at 15:30:00).
#[test]
fn data_end_open_position_bought_on_the_last_session_valued_from_e0() {
    let mut w = base("2400000");
    w.sessions = vec![d(8, 25)];
    let b = book_sess(&w, &[(d(8, 25), "15:40:00")], "");
    let e1 = "2025-08-28-2400000-CE";
    let mut r1: Vec<Sec> = vec![Sec { ts: at(d(8, 25), 9, 45, 0), high: 50_000, low: 1_000, close: 50_000, vol: 75 }];
    r1.extend((0..60).map(|s| sec(d(8, 25), 10, 0, s, 50_000, 100_000)));
    r1.push(sec(d(8, 25), 12, 0, 0, 45_000, 75));
    r1.extend((0..600).map(|s| Sec { ts: at(d(8, 25), 15, 30, 0) + s * 1_000_000, high: 53_000, low: 53_000, close: 53_000, vol: 100_000 }));
    b.put_opt(e1, d(8, 25), Some(r1));
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.streams[0].exit = DATA_END;
    sp.slot_rule = SlotRule::default();
    let r = run(std::slice::from_ref(&b), &sp).unwrap();
    let o = &r.streams[0];
    assert_eq!(kinds(o), vec!["entry"]);
    assert_eq!(o.legs[0].e0, at(d(8, 25), 10, 0, 0));
    let tr = &o.trips[0];
    assert_eq!(tr.leftover, 975);
    assert!(tr.data_end);
    assert_eq!(tr.w0, at(d(8, 25), 10, 0, 0), "W0 = E0: the trip was bought on the last session");
    assert_eq!(tr.bullet1_px, 45_000, "the worst print in [E0, F&O end), never the 09:45 LOW before E0");
    assert_eq!(tr.closed_at, at(d(8, 25), 15, 40, 0));
    assert_eq!(o.flags.get("open_at_data_end_units"), Some(&975));
}

/// The October 2025 world of the Muhurat carry: sessions 10-17, 10-20, 10-21 (the Muhurat session, F&O end 14:45:00), 10-23 (10-22 a
/// holiday), 10-24; the current expiry 10-28 (series 1), the next 11-04 (series 2); each session reads the previous session's file.
fn oct_world() -> World {
    let o = |dd: u32| from_civil(2025, 10, dd);
    let sessions = vec![o(17), o(20), o(21), o(23), o(24)];
    let files = [o(16), o(17), o(20), o(21), o(23)];
    let mut cal = BTreeMap::new();
    for (k, &s) in sessions.iter().enumerate() {
        cal.insert(s, CalRow { file_read: files[k], known_listed: o(28), known_eff: o(28), series: 1, next_eff: Some(from_civil(2025, 11, 4)), d_exp: false });
    }
    let series = vec![SeriesRow { id: 1, aliases: vec![(o(28), o(16), o(28))], true_last: o(28) }, SeriesRow { id: 2, aliases: vec![(from_civil(2025, 11, 4), o(16), from_civil(2025, 11, 4))], true_last: from_civil(2025, 11, 4) }];
    let mut ladder = String::from("file_date\tform\tindex\texpiry\tce_strikes\tpe_strikes\tlots\toi_gcd\toi_rows\tfile_cands\n");
    for file in files {
        for exp in [o(28), from_civil(2025, 11, 4)] {
            let _ = writeln!(ladder, "{}\tudiff\tNIFTY\t{}\t2400000\t2600000\t75\t-\t-\t-", day::fmt(file), day::fmt(exp));
        }
    }
    World { sessions, cal, series, ladder, freeze: FREEZE }
}

/// W4 REPAIR (check-wave4-S9 D-W4-2; plan s4.4 / Q39, S1 MUHURAT-LIST "positions are held through"; the BANKNIFTY 60m SHORT canary's
/// cand 10296): an entry at 2025-10-20 15:15:00 whose exit is decided at 15:30:00 (the F&O end: nothing sells) carries its 975 units.
/// The carry skips the 2025-10-21 Muhurat session (13:45-14:45, prints at Rs 700) and fills from 2025-10-23 09:15:00 (Rs 450), never on
/// 10-21. Red under `m_idx_carry_muhurat` (the final4 carry into the Muhurat session).
#[test]
fn carry_skips_the_muhurat_session() {
    let oct = |dd: u32| from_civil(2025, 10, dd);
    let world = oct_world();
    let fno: Vec<(Day, &str)> = world.sessions.iter().map(|&sd| (sd, if sd == oct(21) { "14:45:00" } else { "15:30:00" })).collect();
    let bk = book_sess(&world, &fno, "day\tweekday\tsegments\tsource\n2025-10-21\tTue\t13:45:00-14:45:00\tSESSIONS-SPECIAL-v1.tsv\n");
    assert!(bk.sess.is_muhurat(oct(21)));
    let con = "2025-10-28-2400000-CE";
    let mut r20: Vec<Sec> = vec![sec(oct(20), 14, 0, 0, 50_000, 75)];
    r20.extend((0..60).map(|k| sec(oct(20), 15, 15, k, 50_000, 100_000)));
    bk.put_opt(con, oct(20), Some(r20));
    bk.put_opt(con, oct(21), Some((0..60).map(|k| sec(oct(21), 13, 45, k, 70_000, 100_000)).collect()));
    bk.put_opt(con, oct(23), Some((0..60).map(|k| sec(oct(23), 9, 15, k, 45_000, 100_000)).collect()));
    let sp = Spec { streams: vec![Stream { book: 0, ce: true, entry: at(oct(20), 15, 15, 0), exit: at(oct(20), 15, 30, 0), slot: 50_000_000, label: "NIFTY CE".into() }], account: 200_000_000, bp: 10_000, kept: false, open_rule: OpenRule::Sequential, target: 50_000_000, slot_rule: SlotRule::default() };
    let res = run(std::slice::from_ref(&bk), &sp).unwrap();
    let s0 = &res.streams[0];
    assert_eq!(kinds(s0), vec!["entry", "exit", "carry"]);
    assert_eq!(s0.legs[0].units, 975);
    assert_eq!(s0.legs[1].units, 0, "the exit at the F&O end sells nothing");
    let cl = &s0.legs[2];
    assert_eq!(cl.day, oct(23), "held through the Muhurat session: the carry fills from the next regular session");
    assert_eq!(cl.decided, at(oct(23), 9, 15, 0));
    assert_eq!(cl.units, 975);
    assert!(cl.fills.iter().all(|fl| fl.0 >= at(oct(23), 9, 15, 0) && fl.3 == 45_000), "every fill from 2025-10-23 09:15:00, never on 10-21");
    assert_eq!(s0.flags.get("carry"), Some(&1));
    assert_eq!(s0.trips[0].sells, vec![(oct(23), 975 * 45_000, 1)]);
    assert_eq!(s0.trips[0].closed_at, cl.known_at);
}

/// W4 REPAIR r3 (check-wave4-S9-r2.md D-R2-1, the checker's exact fix; the NIFTY 60m LONG positional canary's 6 data-end rows hold
/// 2026-11-03 contracts that DP-NEWPX-FAILS lists `unchecked` on 2026-10-30, the last session): a data-end valuation that reads a
/// DATA-law hole is a hole trade. On the PRODUCTION source (`Book::with_newpx`, day files written by the pack's own encoder), the
/// contract's last session 08-26 listed `unchecked` (no store row: the corrected rows kept, the whole session a hole):
/// (A) the sentinel path: no exit before the data end; every unit is valued at 08-26's worst print (Rs 420, the corrected rows) and the
///     trip carries `hole` (its window [09:15:00, 15:40:00) lies on the whole-session hole), counted `data_end_hole` (+ 975 units), while
///     no leg spans a hole (the entry day is clean);
/// (B) the exit path: an exit on the last session sells 375 of 975; the 600 left are valued at the worst print and carry `hole`;
/// (C) the same day files with no sidecar row (a clean contract-day): the same value, no hole, no count;
/// (D) the span test is `walk_order`'s on the valuation window: a trip bought ON the last session (W0 = E0 = 10:00:00) with a failing
///     second at 09:50:00 (before W0) stays clean; one at 12:30:00 (inside [W0, F&O end)) makes it a hole trade; the same value both ways.
/// Red under `m_idx_data_end_nohole` (the span test skipped: the w4r reading).
#[test]
#[allow(clippy::too_many_lines, reason = "one case over the four readings A-D")]
fn data_end_valuation_on_a_hole_session_carries_the_hole() {
    use crate::newpx::Fails;
    use crate::pack::{self, NewSec};
    let root = std::env::temp_dir().join(format!("w4r3-newpx-data-end-hole-{}", std::process::id()));
    let put = |dir: &std::path::Path, day: Day, blocks: &[(&str, Vec<NewSec>)]| {
        let mut raw = Vec::new();
        for (c, s) in blocks {
            raw.extend_from_slice(&pack::enc_block(c, s));
        }
        let ds = day::fmt(day);
        let p = dir.join(&ds[0..7]).join(&ds[0..4]);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join(format!("{ds}.newpx.zst")), pack::zst(&raw)).unwrap();
    };
    let ns = |h: i32, m: i32, s: i32, px: i64, vol: u64| -> NewSec { (h * 3600 + m * 60 + s, px, px, px, px, vol) };
    let fails_of = |rows: &str| {
        let mut f = Fails::default();
        f.add(&format!("contract\tday\tsod\tkind\ts_high\ts_low\ts_close\ts_vol\tt_high\tt_low\tt_close\tt_vol\n{rows}")).unwrap();
        f
    };
    let e1 = "2025-08-28-2400000-CE";
    let mut entry = vec![ns(9, 30, 0, 50_000, 75)];
    entry.extend((0..60).map(|s| ns(10, 0, s, 50_000, 100_000)));
    let unchecked = format!("{e1}\t2025-08-26\t-\tunchecked\n");
    let mut w = base("2400000");
    w.sessions = vec![d(8, 25), d(8, 26)];
    // (A) the sentinel path on the unchecked last session (F&O end 15:40:00; worst print Rs 420 at 11:00:00)
    let dir_a = root.join("a");
    put(&dir_a, d(8, 25), &[(e1, entry.clone())]);
    let mut last = vec![ns(9, 15, 0, 52_000, 75), ns(11, 0, 0, 42_000, 75), ns(14, 0, 0, 55_000, 75)];
    last.extend((0..600).map(|s| ns(15, 30, s, 60_000, 100_000)));
    put(&dir_a, d(8, 26), &[(e1, last)]);
    let fno = [(d(8, 25), "15:30:00"), (d(8, 26), "15:40:00")];
    let mut sp = spec(50_000_000, 10_000, OpenRule::Sequential);
    sp.streams[0].exit = DATA_END;
    let ba = book_sess(&w, &fno, "").with_newpx(NewpxSrc::new(dir_a.clone(), fails_of(&unchecked)));
    let ra = run(std::slice::from_ref(&ba), &sp).unwrap();
    let oa = &ra.streams[0];
    assert_eq!(kinds(oa), vec!["entry"], "no exit leg: valued at the data end");
    assert!(!oa.legs[0].hole, "the entry day is clean: no leg spans a hole");
    let ta = &oa.trips[0];
    assert!(ta.data_end && ta.leftover == 975);
    assert_eq!(ta.w0, at(d(8, 26), 9, 15, 0));
    assert_eq!(ta.bullet1_px, 42_000, "valued at the corrected rows' worst print (unchecked: the rows are kept)");
    assert!(ta.hole, "the valuation window [09:15:00, 15:40:00) lies on the whole-session hole: a hole trade");
    assert_eq!(oa.flags.get("data_end_hole"), Some(&1));
    assert_eq!(oa.flags.get("data_end_hole_units"), Some(&975));
    // (C) the same day files, no sidecar row: a clean contract-day, the same value, no hole, no count
    let bc = book_sess(&w, &fno, "").with_newpx(NewpxSrc::new(dir_a, Fails::default()));
    let rc = run(std::slice::from_ref(&bc), &sp).unwrap();
    let tc = &rc.streams[0].trips[0];
    assert!(tc.data_end && !tc.hole, "a clean last session: no hole");
    assert_eq!((tc.bullet1_px, tc.net, tc.net_per_leg, tc.closed_at), (ta.bullet1_px, ta.net, ta.net_per_leg, ta.closed_at), "the hole labels the trade, it never moves its value");
    assert_eq!(rc.streams[0].flags.get("data_end_hole"), None);
    // (B) the exit path: the data ends on 08-26 (F&O end 15:30:00); the exit at 15:29:00 sells 375 (75 a second for 5 seconds at the 10%
    // level); the 600 left are valued at 08-26's worst print (Rs 400 at 10:00:00) on the unchecked session: a hole trade
    let dir_b = root.join("b");
    put(&dir_b, d(8, 25), &[(e1, entry.clone())]);
    let mut lb = vec![ns(10, 0, 0, 40_000, 75)];
    lb.extend((0..5).map(|s| ns(15, 29, s, 53_000, 750)));
    put(&dir_b, d(8, 26), &[(e1, lb)]);
    let bb = book_sess(&w, &[(d(8, 25), "15:30:00"), (d(8, 26), "15:30:00")], "").with_newpx(NewpxSrc::new(dir_b, fails_of(&unchecked)));
    let mut spb = spec(50_000_000, 1_000, OpenRule::Sequential);
    spb.streams[0].exit = at(d(8, 26), 15, 29, 0);
    let rb = run(std::slice::from_ref(&bb), &spb).unwrap();
    let ob = &rb.streams[0];
    assert_eq!(kinds(ob), vec!["entry", "exit"]);
    assert_eq!(ob.legs[1].units, 375, "75 a second for 5 seconds");
    let tb = &ob.trips[0];
    assert!(tb.data_end && tb.leftover == 600);
    assert_eq!(tb.bullet1_px, 40_000);
    assert!(tb.hole, "the rest's valuation reads the unchecked session: a hole trade");
    assert_eq!(ob.flags.get("data_end_hole"), Some(&1));
    assert_eq!(ob.flags.get("data_end_hole_units"), Some(&600));
    // (D) the span test on the valuation window: the data is 08-25 alone (F&O end 15:40:00) and the trip is bought there (E0 = 10:00:00),
    // so W0 = E0; a failing second (store_only, no corrected second there) at 09:50:00 lies before W0, one at 12:30:00 inside [W0, 15:40:00)
    let dir_d = root.join("d");
    let mut rd: Vec<NewSec> = vec![(9 * 3600 + 45 * 60, 50_000, 50_000, 1_000, 50_000, 75)];
    rd.extend((0..60).map(|s| ns(10, 0, s, 50_000, 100_000)));
    rd.push(ns(12, 0, 0, 45_000, 75));
    rd.extend((0..600).map(|s| ns(15, 30, s, 53_000, 100_000)));
    put(&dir_d, d(8, 25), &[(e1, rd)]);
    let mut wd = base("2400000");
    wd.sessions = vec![d(8, 25)];
    let run_d = |hms: &str| {
        let f = fails_of(&format!("{e1}\t2025-08-25\t{hms}\tstore_only\t50000\t50000\t50000\t75\t-\t-\t-\t-\n"));
        let bk = book_sess(&wd, &[(d(8, 25), "15:40:00")], "").with_newpx(NewpxSrc::new(dir_d.clone(), f));
        run(std::slice::from_ref(&bk), &sp).unwrap()
    };
    let before = run_d("09:50:00");
    let inside = run_d("12:30:00");
    for r in [&before, &inside] {
        let o = &r.streams[0];
        assert_eq!(kinds(o), vec!["entry"]);
        assert_eq!(o.legs[0].e0, at(d(8, 25), 10, 0, 0));
        assert!(!o.legs[0].hole, "the entry walks from 10:00:00: no failing second in its span");
        assert!(o.trips[0].data_end);
        assert_eq!((o.trips[0].w0, o.trips[0].bullet1_px), (at(d(8, 25), 10, 0, 0), 45_000), "W0 = E0; the worst print after it");
    }
    assert!(!before.streams[0].trips[0].hole, "a hole that ends before W0 lies outside the valuation window: no hole");
    assert_eq!(before.streams[0].flags.get("data_end_hole"), None);
    assert!(inside.streams[0].trips[0].hole, "a hole inside [W0, F&O end) makes the data-end trade a hole trade");
    assert_eq!(inside.streams[0].flags.get("data_end_hole_units"), Some(&975));
    assert_eq!(before.streams[0].trips[0].net, inside.streams[0].trips[0].net, "the same value either way");
}
