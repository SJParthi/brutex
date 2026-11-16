//! The extreme-case matrix, red first (plan 12): the host-level named cases on synthetic tapes and the real data rows. Each case
//! names its SN-X id; the mutants m-S16 (no_cut_at.rs), m-S17 (SN-X45) and the kernel / sweep mutants are caught by named tests.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use common_fill::{Dial, SecIn, US_S};
use sniper_host::db::Db;
use sniper_host::price::{self, Cfg, Day, Plan, at_sod, flag};

fn day() -> i32 {
    common_feat::cal::days_from_civil(2024, 3, 1)
}

fn cfg() -> Cfg {
    Cfg {
        dial: Dial::new(&Dial::DEFAULT_BP).unwrap(),
        lev: (5, 1),
        protection_bp: 1000,
        limit_tick: 5,
        square_off_sod: 15 * 3600 + 10 * 60,
        cut_sod: 15 * 3600 + 8 * 60,
        // the cases below were written under the floor rule; SNIPER s16's closest rule has its own case (sn16_*).
        qty_rule: sniper_host::size::QtyRule::Floor,
        account: 200_000_000,
    }
}

fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// A synthetic day 09:15:00..15:30:00: price path from `f(second index)`, volume `vol` a second; an empty span `gap`.
fn tape(f: &dyn Fn(i64) -> i64, vol: u64, gap: Option<(i64, i64)>) -> Day {
    let d = day();
    let open = 9 * 3600 + 15 * 60;
    let end = 15 * 3600 + 30 * 60;
    let mut rows = Vec::new();
    for s in open..end {
        if gap.is_some_and(|(a, b)| s >= a && s < b) {
            continue;
        }
        let p = f(s - open);
        rows.push(SecIn { ts: at_sod(d, s), high: p + 5, low: p - 5, vol });
    }
    let segs = vec![(at_sod(d, open) / US_S, at_sod(d, end) / US_S)];
    Day::new(d, rows, &segs, true, Vec::new()).unwrap()
}

fn slots() -> Vec<i64> {
    vec![200_000_000 / 20, 200_000_000 / 5, 200_000_000 / 100]
}

#[test]
fn sn16_the_closest_count_the_floor_count_and_the_account_cap() {
    // SNIPER s16: a flat Rs 1,000 tape (a buy's reference = the last second's HIGH, 100,005 paisa). Slot Rs 1,00,000 x 5 =
    // Rs 5,00,000 / 1,000.05 = 499.975 shares: floor 499, closest 500 (the 500th share is nearer the target). Slot Rs 4,00,000
    // x 5 / 1,000.05 = 1,999.9: 1,999 / 2,000. Slot Rs 20,000 x 5 / 1,000.05 = 99.995: 99 / 100.
    let t = tape(&|_| 100_000, 1_000_000, None);
    let at = at_sod(day(), 10 * 3600);
    let fl = price::price(&t, &cfg(), at, true, Plan::Clock, &slots()).unwrap();
    let cl = price::price(&t, &Cfg { qty_rule: sniper_host::size::QtyRule::Closest, ..cfg() }, at, true, Plan::Clock, &slots()).unwrap();
    assert_eq!(fl.refpx, 100_005);
    assert_eq!(fl.sized.iter().map(|s| s.qty).collect::<Vec<_>>(), vec![499, 1999, 99]);
    assert_eq!(cl.sized.iter().map(|s| s.qty).collect::<Vec<_>>(), vec![500, 2000, 100]);
    for (ni, q) in [500_u64, 2000, 100].iter().enumerate() {
        for l in 0..5 {
            assert_eq!(cl.lv[ni][l].units, *q, "every level fills the closest count on a deep tape");
            assert_eq!(cl.lv[ni][l].flags & flag::ACCOUNT_CAP, 0);
            assert!(cl.lv[ni][l].entry_value > 0 && cl.lv[ni][l].exit_value > 0);
        }
    }
    // The fills' prices are the same blocks: the closing value per unit equals the floor run's (a flat tape).
    assert_eq!(cl.lv[0][4].exit_value * 499, fl.lv[0][4].exit_value * 500);
    // Base Q7: an account of exactly Rs 1,00,000 cannot pay the 500th share's money in (Rs 1,00,005 > Rs 1,00,000): the largest
    // count that fits (499) is taken and named on every level (ACCOUNT_CAP); the floor rule never needs more than the slot.
    let small = Cfg { qty_rule: sniper_host::size::QtyRule::Closest, account: 10_000_000, ..cfg() };
    let c1 = price::price(&t, &small, at, true, Plan::Clock, &[10_000_000]).unwrap();
    assert_eq!(c1.sized[0], sniper_host::size::Sized { qty: 499, rule_qty: 500, capped: true });
    assert!(c1.lv[0].iter().take(5).all(|x| x.flags & flag::ACCOUNT_CAP != 0 && x.units == 499));
    let f1 = price::price(&t, &Cfg { account: 10_000_000, ..cfg() }, at, true, Plan::Clock, &[10_000_000]).unwrap();
    assert_eq!(f1.sized[0], sniper_host::size::Sized { qty: 499, rule_qty: 499, capped: false });
    // An account that buys no share: refused, named (AccountZero); a slot whose target is at most half a share: ZeroQty.
    let none = Cfg { qty_rule: sniper_host::size::QtyRule::Closest, account: 10_000, ..cfg() };
    assert_eq!(price::price(&t, &none, at, true, Plan::Clock, &[10_000_000]).unwrap_err(), price::Refusal::AccountZero);
    let half = Cfg { qty_rule: sniper_host::size::QtyRule::Closest, ..cfg() };
    assert_eq!(price::price(&t, &half, at, true, Plan::Clock, &[10_000]).unwrap_err(), price::Refusal::ZeroQty, "10,000 x 5 = 50,000 < half a share (50,002.5): 0, refused");
    assert_eq!(price::price(&t, &half, at, true, Plan::Clock, &[10_001]).unwrap().sized[0].qty, 1, "50,005 > half a share: 1 (the exact tie is size.rs's unit case)");
    assert_eq!(price::price(&t, &cfg(), at, true, Plan::Clock, &[10_001]).unwrap_err(), price::Refusal::ZeroQty, "the floor rule buys no share below one share");
}

#[test]
fn sn_x44_a_winning_stop_rewalks_the_entry_s2() {
    // S2: the price falls after a 10:00 long entry; the stop wins at A; that level's entry is re-walked with stop A (at most one
    // re-walk per winning level, so at most 5 an order); the exit is at A, never through FillPath::cut_at.
    let t = tape(&|i| if i < 2700 { 100_000 } else { 100_000 - (i - 2700) * 3 }, 400, None);
    let p = price::price(&t, &cfg(), at_sod(day(), 10 * 3600), true, Plan::Stop { x: 300 }, &slots()).unwrap();
    assert!(p.rewalks >= 1 && p.rewalks <= 15, "rewalks {} (at most 5 per order, 3 orders)", p.rewalks);
    for lv in &p.lv {
        for x in lv.iter().take(5) {
            assert!(x.flags & flag::REWALKED != 0, "every level's stop won: re-walked");
            assert!(x.exit_anchor < at_sod(day(), 15 * 3600 + 600), "the stop's anchor is before the square-off");
            assert!(x.net < 0, "a falling long loses");
        }
    }
}

#[test]
fn sn_x44_a_zero_unit_entry_sends_no_exit_order() {
    // No volume after the decision until the square-off: the entry fills 0 units; no exit order; counted ZERO; net 0.
    let t = tape(&|_| 100_000, 300, Some((10 * 3600, 15 * 3600 + 30 * 60)));
    let p = price::price(&t, &cfg(), at_sod(day(), 10 * 3600), true, Plan::Clock, &slots()).unwrap();
    for x in p.lv[0].iter().take(5) {
        assert!(x.flags & flag::ZERO != 0);
        assert_eq!((x.net, x.units), (0, 0));
    }
}

#[test]
fn sn_x49_g_comp_the_derived_composite_equals_the_priced_one() {
    // S11 / G-COMP: on seeded random tapes, the composite derived from the stored XS and XQ-A rows (the earlier exit anchor wins
    // per level, a tie to the stop) equals the composite priced through crate v2, level by level.
    for seed in 1..=8_u64 {
        let f = move |i: i64| {
            let mut x = 100_000_i64;
            let mut st = seed;
            for _ in 0..(i / 30) {
                st = mix(st);
                x += i64::try_from(st % 41).unwrap() - 20;
            }
            x
        };
        let t = tape(&f, 250 + seed * 37, None);
        let td = at_sod(day(), 10 * 3600);
        let s_anchor = at_sod(day(), 11 * 3600 + i64::try_from(seed).unwrap() * 120);
        let xs = price::price(&t, &cfg(), td, true, Plan::Signal { anchor: s_anchor, immediate: false }, &slots()).unwrap();
        let xq = price::price(&t, &cfg(), td, true, Plan::Stop { x: 150 }, &slots()).unwrap();
        let comp = price::price(&t, &cfg(), td, true, Plan::StopOrSignal { x: 150, anchor: s_anchor, immediate: false }, &slots()).unwrap();
        let der = price::derive_composite(&xs, &xq);
        assert_eq!(der.lv, comp.lv, "seed {seed}");
        assert_eq!(der.entry_first, comp.entry_first);
        // The stored rows encode identically (the 112-B layout).
        let origin = at_sod(common_feat::cal::days_from_civil(2018, 1, 1), 0);
        assert_eq!(price::encode(&der, 5, origin).unwrap(), price::encode(&comp, 5, origin).unwrap());
        assert_eq!(price::encode(&comp, 5, origin).unwrap().len(), 112);
    }
}

#[test]
fn sn_x12_an_order_larger_than_its_blocks_moves_the_rest() {
    // N = 5 at Rs 4 L x 5 against 20 shares a second: the rest moves block by block (crate S7).
    let t = tape(&|_| 100_000, 20, None);
    let p = price::price(&t, &cfg(), at_sod(day(), 10 * 3600), true, Plan::Signal { anchor: at_sod(day(), 12 * 3600), immediate: false }, &slots()).unwrap();
    let n5_l10 = p.lv[1][0];
    assert!(n5_l10.units > 0);
    assert!(p.steps > 100, "many blocks walked");
}

#[test]
fn sn_x13_a_decision_at_15_08_is_rejected() {
    let t = tape(&|_| 100_000, 300, None);
    let r = price::price(&t, &cfg(), at_sod(day(), 15 * 3600 + 8 * 60), true, Plan::Clock, &slots());
    assert_eq!(r, Err(price::Refusal::AfterCut));
}

#[test]
fn sn_x14_square_off_and_unclosable_units() {
    // X-1510: the exit is the square-off from 15:10:00; with no print after 15:10 the units are unclosable in session (S15).
    let t = tape(&|_| 100_000, 300, Some((15 * 3600 + 10 * 60, 15 * 3600 + 30 * 60)));
    let p = price::price(&t, &cfg(), at_sod(day(), 14 * 3600), true, Plan::Clock, &slots()).unwrap();
    let x = p.lv[0][4];
    assert_eq!(x.exit_anchor, at_sod(day(), 15 * 3600 + 10 * 60));
    assert!(x.flags & flag::UNCLOSABLE != 0 && x.unclosed > 0);
}

#[test]
fn sn_x45_the_money_cap_law_is_refused_at_load_s3() {
    let e = sniper_host::law::load(common_fill::LAW_STD_CAP).unwrap_err();
    assert_eq!(e, "with money cap: not run (SNIPER s8: no limit or wait-for-a-dip entry in any row)");
    assert!(sniper_host::law::load(common_fill::LAW_STD).is_ok());
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sniper-host-test-{name}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

#[test]
fn db_writer_once_labels_and_the_hash_chain() {
    let d = tmp("db");
    let db = Db::open(&d, "test").unwrap();
    let rows = vec![vec!["a".to_string(), "S12 label".to_string()]];
    db.write("T1.tsv", &["x", "labels"], &rows).unwrap();
    assert!(db.write("T1.tsv", &["x", "labels"], &rows).is_err(), "written once");
    assert!(db.write("T2.tsv", &["x", "labels"], &[vec!["a".to_string(), String::new()]]).is_err(), "a row without its standing labels is refused");
    assert!(db.write("T3.tsv", &["x"], &[vec!["a".to_string()]]).is_err(), "a table without a labels column is refused");
    db.write_bin("P1.bin", &[1, 2, 3]).unwrap();
    assert_eq!(db.verify().unwrap(), 2);
    std::fs::write(d.join("P1.bin"), [9, 9]).unwrap();
    assert!(db.verify().is_err(), "a changed file breaks the audit");
}

#[test]
fn sn_x39_g_inc_a_day_appended_equals_the_full_run() {
    // S9 / G-INC: appending days one by one gives the same partition bytes, test sums and twin draws as one full run.
    let recs: Vec<Vec<u8>> = (0..12_u8).map(|i| vec![i; 112]).collect();
    let d1 = tmp("full");
    let full = Db::open(&d1, "t").unwrap();
    sniper_host::newday::append_day(&full, "ALL.bin", 112, &recs).unwrap();
    let d2 = tmp("inc");
    let inc = Db::open(&d2, "t").unwrap();
    let mut cat = Vec::new();
    for (i, r) in recs.iter().enumerate() {
        sniper_host::newday::append_day(&inc, &format!("D{i:02}.bin"), 112, std::slice::from_ref(r)).unwrap();
        cat.extend(std::fs::read(d2.join(format!("D{i:02}.bin"))).unwrap());
    }
    assert_eq!(cat, std::fs::read(d1.join("ALL.bin")).unwrap());
    assert!(sniper_host::newday::append_day(&inc, "D00.bin", 112, &recs[..1]).is_err(), "never over an earlier partition");
    let mut a = sniper_host::newday::TestSums::default();
    let mut b = sniper_host::newday::TestSums::default();
    for i in 0..12 {
        a.add(2023 * 12 + i, 100 * i64::from(i)).unwrap();
    }
    for i in (0..12).rev() {
        b.add(2023 * 12 + i, 100 * i64::from(i)).unwrap();
    }
    assert_eq!(a, b);
    // Twin draws: a session appended to the open year never moves an earlier draw's key; the draw set is a pure function.
    use sniper_host::twin::{Slot, draws};
    let slots: Vec<Slot> = (0..40).map(|s| Slot { sess: s, year: 2026, candles: 117 }).collect();
    let mut py = BTreeMap::new();
    py.insert(2026, 5);
    let before = draws(20_261_113, 7, &slots, &py);
    let mut more = slots.clone();
    more.push(Slot { sess: 40, year: 2026, candles: 117 });
    let after = draws(20_261_113, 7, &more, &py);
    assert_eq!(after, draws(20_261_113, 7, &more, &py), "deterministic");
    let kept: Vec<_> = before.iter().filter(|x| after.contains(x)).collect();
    assert!(kept.len() >= 4, "at most one earlier draw is displaced by one appended session");
}

#[test]
fn query_lookup_is_o1_and_round_trips() {
    let keys = vec![(0_u32, 1_u16), (0, 5), (3, 2)];
    let ix = sniper_host::query::build_index(&keys, 4, 117);
    assert_eq!(sniper_host::query::lookup(&ix, 0, 5), Some(1));
    assert_eq!(sniper_host::query::lookup(&ix, 3, 2), Some(2));
    assert_eq!(sniper_host::query::lookup(&ix, 2, 2), None);
    assert_eq!(sniper_host::query::lookup(&ix, 9, 2), None);
    let part: Vec<u8> = (0..3_u8).flat_map(|i| vec![i; 112]).collect();
    assert_eq!(sniper_host::query::record(&part, 112, 2).unwrap()[0], 2);
}

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

#[test]
fn the_data_rows_load_fail_closed() {
    // Gate 1 reads: RUN rows, WORDS, VIEWS, common_wf's WINDOWS rows, the dials file's core ladders, the 21 base exit sets.
    let inp = sniper_host::esn1::load_inputs(&data()).unwrap();
    assert_eq!(inp.exits.len(), 21);
    assert_eq!(inp.core.0.l[0].values(), 32);
    assert_eq!(inp.core.0.l[1].values(), 568);
    assert_eq!(inp.core.0.l[2].values(), 22);
    assert_eq!(inp.core.1.l[0].levels.first(), Some(&30), "the short RSI ladder is the mirror (30 .. 0)");
    assert_eq!(inp.run.num::<usize>("threads").unwrap(), 14, "SNIPER s14: the thread count is a data row");
    let wf = &inp.windows.rows[0];
    assert_eq!((sniper_host::rows::mk_text(wf.1), sniper_host::rows::mk_text(wf.2)), ("2018-09".to_string(), "2019-12".to_string()));
    let tt = &inp.windows.rows[1];
    assert_eq!(sniper_host::rows::mk_text(tt.2), "2022-12");
    assert!(sniper_host::rows::RunRows::parse("key\tvalue\tsource\nx\t1\t\n").is_err(), "a row without a source is refused");
    // SN-X50 (S6): a long's CPR / pivot dials hold no low zone.
    for d in inp.dials.iter().filter(|d| d.id.starts_with("L-PIV") || d.id == "L-CPR" || d.id == "L-CPRWK" || d.id == "L-CPRMO") {
        assert!(!d.values.contains("BELOW"), "{}: a long holds no reversal zone (S6)", d.id);
    }
}

#[test]
fn the_standing_labels_and_trade_count_words() {
    // S10: every kid-page row carries the s12 line, the fixed_after words, the proof step and the standing labels.
    let inp = sniper_host::esn1::load_inputs(&data()).unwrap();
    let st = inp.words.standing(1, "printed by the tool");
    assert!(st[0].starts_with("the instrument and its pattern were chosen from the operator's chart of 2026-04"));
    assert_eq!(st[1], "the train/test row was fixed after walk-forward verdicts on its test years had been shown (design-level hindsight)");
    assert_eq!(st[2], "proof step reached: 1 of 8 (printed by the tool)");
    assert!(st.iter().any(|l| l == "with money cap: not run (SNIPER s8: no limit or wait-for-a-dip entry in any row)"));
    // SNIPER s16: the tree's RUN rows are SNIPER-RUN-v5.tsv with the qty_rule row (closest for every NEW run); fail closed.
    assert_eq!(inp.qty_rule, sniper_host::size::QtyRule::Closest);
    // SECONDS PORTER (secf): the tree's data folder also holds SNIPER-RUN-v6.tsv (v5 + the seconds rows), which is read when present;
    // a folder with v5 only (the sizing builder's data-closest) still loads v5. The qty_rule row is in both.
    assert!(inp.shas.iter().any(|(f, _)| f == "SNIPER-RUN-v6.tsv"));
    let only5 = std::env::temp_dir().join(format!("sniper-secf-only-v5-{}", std::process::id()));
    std::fs::create_dir_all(&only5).unwrap();
    for e in std::fs::read_dir(data()).unwrap() {
        let p = e.unwrap().path();
        if p.is_file() && p.file_name().is_some_and(|n| n != "SNIPER-RUN-v6.tsv") {
            std::fs::copy(&p, only5.join(p.file_name().unwrap())).unwrap();
        }
    }
    let inp5 = sniper_host::esn1::load_inputs(&only5).unwrap();
    assert!(inp5.shas.iter().any(|(f, _)| f == "SNIPER-RUN-v5.tsv"));
    assert_eq!(inp5.qty_rule, sniper_host::size::QtyRule::Closest);
    assert!(inp5.run.get("seconds_family").is_err(), "v5 has no seconds rows");
    assert_eq!(inp.run.get("seconds_family").unwrap(), "3s,5s,10s,15s,30s");
    let empty = std::env::temp_dir().join(format!("sniper-sz-no-v5-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    assert!(sniper_host::esn1::load_inputs(&empty).is_err(), "a folder without SNIPER-RUN-v5.tsv is refused");
    let w = inp.words.trade_count(3, 3, 7);
    assert_eq!(w.len(), 3, "counts + too few unseen + too few learning");
    assert_eq!(w[1], "only 7 unseen trades: too few to call anything proven");
    assert_eq!(inp.words.trade_count(40, 40, 40).len(), 1);
}

#[test]
fn the_red_door_refuses_a_red_build() {
    if cfg!(feature = "red") || cfg!(feature = "m_s16") || cfg!(feature = "m_s17") {
        assert!(sniper_host::refuse_if_red().is_err());
    } else {
        assert!(sniper_host::refuse_if_red().is_ok(), "{}", sniper_host::law_line());
    }
}

#[test]
fn the_launch_gate_parses_swap() {
    assert_eq!(sniper_host::monitor::parse_swap("vm.swapusage: total = 36864.00M  used = 35966.81M  free = 897.19M  (encrypted)"), Some((35966, 897)));
}

#[test]
fn sn_x23_the_sweep_holds_one_first_trade_per_session() {
    // SN-X23 / B1 (the host's red reading = CUBE's suffix reading): two qualifying candles a session; the later candle (CCI
    // bucket 3) enters first, the earlier one (bucket 1) replaces it: one removal per session, and no list entry holds more
    // trades than the window has sessions.
    use sniper_kernel::line::{Core, Row};
    use wf_sweep::acc::{Layout, Outcome};
    let mk = |y: i32, m: i32| y * 12 + m - 1;
    let mut sess_mk = Vec::new();
    for m in mk(2018, 9)..=mk(2022, 12) {
        sess_mk.push(m);
        sess_mk.push(m);
    }
    let n = sess_mk.len();
    let mut rows = Vec::new();
    let mut out = Vec::new();
    for (s, &m) in sess_mk.iter().enumerate() {
        let s32 = u32::try_from(s).unwrap();
        for (k, cb) in [(1_u16, 1_u16), (2, 3)] {
            rows.push(Row { sess: s32, k: u32::from(k), rb: 0, cb, wb: 0 });
            out.push(Outcome { sess: s32, mk: m, id: rows.len() as u64, net: [1000; wf_sweep::MAXC], rd_bp: [10; wf_sweep::MAXC] });
        }
    }
    let wins = vec![("WF".to_string(), mk(2018, 9), mk(2019, 12)), ("TT".to_string(), mk(2018, 9), mk(2022, 12))];
    let lay = Layout::new(&sess_mk, (mk(2018, 9), mk(2022, 12)), &wins).unwrap();
    let views = wf_sweep::views::parse("WF-R73\tWF\t1\tTHE\nTT-R73\tTT\t1\tTHE\n", &["WF".to_string(), "TT".to_string()]).unwrap();
    let radix = vec![1_u64, 4, 1];
    let ctx = sniper_host::sweep::Ctx { rows: &rows, out: &out, lay: &lay, views: &views, cols: 5, core: Core { nr: 1, nc: 4, nw: 1 }, sessions: n, top: 1_000_000, radix: &radix, core_pos: [0, 1, 2], filter: None };
    let res = sniper_host::sweep::run(&ctx, &[], 2).unwrap();
    assert!(res.refused.is_empty(), "{:?}", res.refused);
    assert_eq!(res.walk.removes, n as u64, "one FIRST-mode move per session");
    for (li, l) in res.lists.iter().enumerate() {
        let sessions = if li / 5 == 0 { 32 } else { n as i64 };
        for (k, _, _) in l.entries() {
            assert!(k.0[5] <= sessions, "list {li}: {} trades > {sessions} sessions", k.0[5]);
        }
        assert!(!l.is_empty(), "every cell is eligible (all nets positive)");
    }
}

#[test]
fn sn_b2_the_protection_hold_is_flagged_and_counted_per_pick_s3() {
    // HOST REPAIR B2 (check-build-host-v1; S3 "Dhan's market protection ... counted per pick"): right after a 10:00 long decision
    // the price prints 15% above the reference for 20 s (past Dhan's 10% protection LIMIT), then returns. The entry's first blocks
    // are held (the crate's `past_limit`) and the units fill later: every level of every N carries PROTECTED and none CLIPPED (a
    // block cut by the order's own stop is a different fact; Plan::Clock fills the entry long before its stop). The per-pick
    // count is 1 of the trades that carry it. Mutant m_b2 (the count reads CLIPPED, the check text's field) fails here.
    let t = tape(&|i| if (2700..2720).contains(&i) { 115_000 } else { 100_000 }, 400, None);
    let p = price::price(&t, &cfg(), at_sod(day(), 10 * 3600), true, Plan::Clock, &slots()).unwrap();
    for lv in &p.lv {
        for x in lv.iter().take(5) {
            assert!(x.flags & flag::PROTECTED != 0, "the protection hold is flagged: flags {:#x}", x.flags);
            assert_eq!(x.flags & flag::CLIPPED, 0, "no stop clip on this trade");
            assert!(x.units > 0, "the held units fill after the spike");
        }
    }
    let f = p.lv[0][2].flags;
    assert_eq!(price::held_counts([f, 0, f].into_iter()), (2, 0), "protection held on 2 of 3 trades; a stop clip on 0");
    assert_eq!(price::held_counts([flag::CLIPPED].into_iter()), (0, 1), "a stop clip alone is no protection hold");
    // a calm tape: no hold anywhere
    let calm = tape(&|_| 100_000, 400, None);
    let q = price::price(&calm, &cfg(), at_sod(day(), 10 * 3600), true, Plan::Clock, &slots()).unwrap();
    assert!(q.lv.iter().all(|lv| lv.iter().take(5).all(|x| x.flags & flag::PROTECTED == 0)), "no hold on a calm tape");
}

#[test]
fn sn_b3_sniper_q_reads_the_352_byte_store_with_its_own_width() {
    // HOST REPAIR B3 (check-build-host-v1): two 11-N x 5-level records (352 B each: the width of every OUT store this host writes)
    // in a part named as the host names it (no "-N55"). `sniper-q DIR PART - 1` must print record 1's nets, each with its N and
    // level, and never a byte of record 0. Mutant m_b3 (the width guessed from the name: 112 B) reads record 0's bytes: caught.
    assert_eq!(price::record_width(11, 5), 352);
    assert_eq!(price::record_width(3, 5), 112);
    let mk = |base: i64| price::Priced {
        lv: (0..11_usize)
            .map(|ni| {
                let mut a = [price::LevelOut::default(); common_fill::MAXL];
                for (l, x) in a.iter_mut().enumerate().take(5) {
                    x.net = base + i64::try_from(ni * 10 + l).unwrap();
                }
                a
            })
            .collect(),
        ..price::Priced::default()
    };
    let r0 = price::encode(&mk(-1_000_000), 5, 0).unwrap();
    let r1 = price::encode(&mk(7_000_000), 5, 0).unwrap();
    assert_eq!((r0.len(), r1.len()), (352, 352), "encode writes record_width(11, 5) bytes");
    let dir = tmp("sniper-q-b3");
    std::fs::write(dir.join("OUT-3min-LONG-XS-10-30.bin"), [r0, r1].concat()).unwrap();
    let q = |part: &str, rec: &str| std::process::Command::new(env!("CARGO_BIN_EXE_sniper-q")).arg(&dir).arg(part).arg("-").arg(rec).arg("--data").arg(data()).output().unwrap();
    let out = q("OUT-3min-LONG-XS-10-30.bin", "1");
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(out.status.success(), "sniper-q failed: {}", String::from_utf8_lossy(&out.stderr));
    assert!(s.contains("record 1 of 2 (352 B each"), "the width and the record count: {s}");
    assert!(s.contains("N=5: [7000000, 7000001, 7000002, 7000003, 7000004]"), "record 1's N = 5 nets: {s}");
    assert!(s.contains("N=100: [7000100, 7000101, 7000102, 7000103, 7000104]"), "record 1's N = 100 nets: {s}");
    assert!(!s.contains("-99"), "no byte of record 0: {s}");
    // a part that is not a whole number of records is refused, by name
    std::fs::write(dir.join("OUT-odd.bin"), vec![0_u8; 352 + 112]).unwrap();
    let bad = q("OUT-odd.bin", "0");
    assert!(!bad.status.success(), "a part of 464 bytes is refused");
    assert!(String::from_utf8_lossy(&bad.stderr).contains("not a whole number of 352-byte records"));
}

// ===================================================================================================================================
// SNIPER s20 (operator Sat 14 Nov 2026 ~13:2x IST, "Supertrend-or-5-min combined"; MAIN 5b507203 13:33 IST): the ONE combined exit
// XC = the first of the Supertrend flip (the XS signal anchor) and a plain clock 300 s after E0 (crate Rule::TimeExit), the square-off.
// ===================================================================================================================================

fn closest() -> Cfg {
    Cfg { qty_rule: sniper_host::size::QtyRule::Closest, ..cfg() }
}

/// The level of `a` with REWALKED cleared (the clock's S2 re-walk is the one flag a signal plan at the clock's anchor lacks).
fn no_rw(mut x: price::LevelOut) -> price::LevelOut {
    x.flags &= !flag::REWALKED;
    x
}

fn seeded(seed: u64) -> impl Fn(i64) -> i64 {
    move |i: i64| {
        let mut x = 100_000_i64;
        let mut st = seed;
        for _ in 0..(i / 30) {
            st = mix(st);
            x += i64::try_from(st % 41).unwrap() - 20;
        }
        x
    }
}

#[test]
fn sn20_the_exit_sets_v2_and_v3_fail_closed() {
    use sniper_host::rows::{ExitKind, exit_sets};
    let v2 = std::fs::read_to_string("/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/sniper-req76-20261113/plan/SNIPER-EXITS-v2.plan.tsv").unwrap();
    let v3 = std::fs::read_to_string(data().join("SNIPER-EXITS-v3.plan.tsv")).unwrap();
    let a = exit_sets(&v2).unwrap();
    let b = exit_sets(&v3).unwrap();
    assert_eq!(a.len(), 21, "v2: the 21 base sets, no XC row");
    assert!(a.iter().all(|e| !matches!(e.kind, ExitKind::SupertrendOrClock(..))));
    assert_eq!(b.len(), 22, "v3: the 21 base sets + the ONE combined row");
    assert_eq!(&b[..21], &a[..], "v3's base sets are v2's, in order");
    assert_eq!(b[21].id, "XC-ST10-30-T300");
    assert_eq!(b[21].kind, ExitKind::SupertrendOrClock(10, 3000, 300), "Supertrend 10 x 3.0 (XS-10-30's row) or 300 s");
    let xs1030 = a.iter().find(|e| e.id == "XS-10-30").unwrap();
    assert_eq!(xs1030.kind, ExitKind::Supertrend(10, 3000), "the same Supertrend row as XS-10-30");
    // Fail closed: every malformed XC row is a named refusal, never a default.
    let row = v3.lines().last().unwrap().to_string();
    let with = |new_row: &str| format!("{}\n{new_row}\n", v3.lines().take(v3.lines().count() - 1).collect::<Vec<_>>().join("\n"));
    for bad_id in ["XC-ST10-30", "XC-ST10-30-300", "XC-ST10-30-T", "XC-ST10-30-T300-1", "XC-ST08-30-T300", "XC-ST10-25-T300", "XC-ST10-30-T0", "XC-ST10-30-T3601", "XC-ST1O-30-T300", "XC-ST10-+30-T300"] {
        let r = row.replacen("XC-ST10-30-T300", bad_id, 1);
        assert!(exit_sets(&with(&r)).is_err(), "{bad_id} must be refused");
    }
    assert!(exit_sets(&with(&row.replacen("\tCOMBINED\t", "\tSUPERTREND\t", 1))).is_err(), "an XC row's kind cell must be COMBINED");
    assert!(exit_sets(&format!("{v3}{row}\n")).is_err(), "a repeated id is refused");
    let no_base: String = v3.lines().filter(|l| !l.starts_with("XE-200\t")).map(|l| format!("{l}\n")).collect();
    assert!(exit_sets(&no_base).unwrap_err().contains("20 base sets, not 21"), "the base count stays 21 (XC rows do not count)");
}

#[test]
fn sn20_the_flip_wins_when_it_comes_first() {
    // The flip's anchor (T + 120 s) comes before E0 + 300 s (E0 = T on a deep tape): every level is EXACTLY the XS plan's
    // (Plan::Signal at that anchor), no re-walk; F17's immediate flag as XS. Mutant m_xc_flip (the flip ignored) exits at the
    // clock instead: caught.
    for seed in 1..=6_u64 {
        let t = tape(&seeded(seed), 300 + seed * 41, None);
        let td = at_sod(day(), 10 * 3600 + i64::try_from(seed).unwrap() * 60);
        for long in [true, false] {
            for immediate in [false, true] {
                let anchor = td + 120 * US_S;
                let xs = price::price(&t, &closest(), td, long, Plan::Signal { anchor, immediate }, &slots()).unwrap();
                let xc = price::price(&t, &closest(), td, long, Plan::ClockOrSignal { secs: 300, anchor, immediate }, &slots()).unwrap();
                assert_eq!(xc.lv, xs.lv, "seed {seed} long {long} immediate {immediate}: the flip decides every level");
                assert_eq!(xc.entry_first, xs.entry_first);
                assert_eq!(xc.rewalks, 0);
                for lv in &xc.lv {
                    for x in lv.iter().take(5) {
                        assert_eq!(x.exit_anchor, anchor);
                        assert_eq!(x.flags & flag::IMMEDIATE != 0, immediate, "F17 flagged exactly when the flip was immediate");
                    }
                }
                let origin = at_sod(common_feat::cal::days_from_civil(2018, 1, 1), 0);
                assert_eq!(price::encode(&xc, 5, origin).unwrap(), price::encode(&xs, 5, origin).unwrap());
            }
        }
    }
}

#[test]
fn sn20_the_clock_wins_even_in_profit() {
    // A long on a rising tape (a short on a falling one) is in profit at E0 + 300 s: the go-positive timer (XQ-G300) would hold,
    // the plain clock closes. The flip comes an hour later. Every level exits at max(E0 + 300 s, E1) = T + 300 s, re-walked (S2),
    // and equals the XS plan anchored at that instant (the same entry fills up to it, the same exit order), REWALKED aside; F17's
    // flag is never set on a level the clock decided. Mutant m_xc_clock (the clock read as go-positive) holds to the flip: caught.
    for long in [true, false] {
        let f = move |i: i64| if long { 100_000 + i * 2 } else { 200_000 - i * 2 };
        let t = tape(&f, 500, None);
        let td = at_sod(day(), 11 * 3600);
        let flip = td + 3600 * US_S;
        let clock = td + 300 * US_S;
        let xc = price::price(&t, &closest(), td, long, Plan::ClockOrSignal { secs: 300, anchor: flip, immediate: true }, &slots()).unwrap();
        let at_clock = price::price(&t, &closest(), td, long, Plan::Signal { anchor: clock, immediate: false }, &slots()).unwrap();
        let gp = price::price(&t, &closest(), td, long, Plan::Timer { secs: 300 }, &slots()).unwrap();
        assert_eq!(xc.entry_first, td, "E0 = T on a deep tape");
        for (ni, lv) in xc.lv.iter().enumerate() {
            for (l, x) in lv.iter().enumerate().take(5) {
                assert_eq!(x.exit_anchor, clock, "long {long} N#{ni} level {l}: the plain clock at E0 + 300 s");
                assert_ne!(x.flags & flag::REWALKED, 0, "the clock's win re-walks the entry (S2)");
                assert_eq!(x.flags & flag::IMMEDIATE, 0, "the clock decided: no F17 flag");
                assert_eq!(no_rw(*x), at_clock.lv[ni][l], "the XS plan at the clock's anchor, REWALKED aside");
                assert!(x.net > 0, "in profit at the clock");
                assert_ne!(gp.lv[ni][l].exit_anchor, clock, "the go-positive timer holds a position in profit (the case tells the two apart)");
            }
        }
        assert_eq!(xc.rewalks, 15, "one re-walk per level per N (3 N x 5 levels)");
    }
}

#[test]
fn sn20_the_square_off_the_tie_and_thin_tapes() {
    // (1) No flip before 15:10:00 and the clock after it (T 15:06:00, E0 + 300 s = 15:11:00): exactly the square-off plan.
    let t = tape(&seeded(9), 400, None);
    let sq = at_sod(day(), 15 * 3600 + 10 * 60);
    let td = at_sod(day(), 15 * 3600 + 6 * 60);
    let xc = price::price(&t, &closest(), td, true, Plan::ClockOrSignal { secs: 300, anchor: sq, immediate: false }, &slots()).unwrap();
    let ck = price::price(&t, &closest(), td, true, Plan::Clock, &slots()).unwrap();
    assert_eq!(xc.lv, ck.lv, "the 15:10 square-off backstop");
    // (2) The tie: the flip's anchor IS E0 + 300 s (E0 = T): the crate's time exit fires only strictly before the signal
    // anchor, so the flip's plan prices the level (the same instant, the same fills either way); no re-walk.
    let td = at_sod(day(), 10 * 3600);
    let anchor = td + 300 * US_S;
    let xc = price::price(&t, &closest(), td, false, Plan::ClockOrSignal { secs: 300, anchor, immediate: true }, &slots()).unwrap();
    let xs = price::price(&t, &closest(), td, false, Plan::Signal { anchor, immediate: true }, &slots()).unwrap();
    assert_eq!(xc.lv, xs.lv);
    assert_eq!(xc.rewalks, 0);
    // (3) A thin tape (a print every 7th second, 3 shares a second at most): levels fill at different speeds, so E0 and E1 differ
    // by level; per level the exit is the clock's max(E0 + 300 s, E1) when it is at or before the flip anchor (then the XS plan
    // at that anchor, REWALKED aside), else the flip's own level. Checked against both plans on several anchors.
    let thin = |i: i64| 100_000 + (i % 13) * 5;
    let mut rows = Vec::new();
    let d = day();
    for s in (9 * 3600 + 15 * 60)..(15 * 3600 + 30 * 60) {
        if s % 7 == 0 {
            let p = thin(s);
            rows.push(SecIn { ts: at_sod(d, s), high: p + 5, low: p - 5, vol: u64::try_from(1 + s % 3).unwrap() });
        }
    }
    let segs = vec![(at_sod(d, 9 * 3600 + 15 * 60) / US_S, at_sod(d, 15 * 3600 + 30 * 60) / US_S)];
    let th = Day::new(d, rows, &segs, true, Vec::new()).unwrap();
    let small = [2_000_000_i64, 400_000];
    let mut clock_won = 0;
    let mut flip_won = 0;
    for off in [200_i64, 290, 300, 301, 302, 305, 310, 330, 400, 900] {
        let anchor = td + off * US_S;
        let xc = price::price(&th, &closest(), td, true, Plan::ClockOrSignal { secs: 300, anchor, immediate: false }, &small).unwrap();
        let xs = price::price(&th, &closest(), td, true, Plan::Signal { anchor, immediate: false }, &small).unwrap();
        for ni in 0..small.len() {
            for l in 0..5 {
                let x = xc.lv[ni][l];
                if x.exit_anchor < anchor {
                    clock_won += 1;
                    assert!(x.exit_anchor >= td + 300 * US_S, "the clock never before T + 300 s");
                    assert_ne!(x.flags & flag::REWALKED, 0);
                    let at = price::price(&th, &closest(), td, true, Plan::Signal { anchor: x.exit_anchor, immediate: false }, &small).unwrap();
                    assert_eq!(no_rw(x), at.lv[ni][l], "off {off} N#{ni} level {l}: the XS plan at the clock's anchor");
                } else if x.flags & flag::REWALKED == 0 {
                    flip_won += 1;
                    assert_eq!(x, xs.lv[ni][l], "off {off} N#{ni} level {l}: the flip's own level");
                } else {
                    // the clock's anchor max(E0 + 300 s, E1) landed exactly on the flip anchor: the same fills as the flip's level.
                    assert_eq!(no_rw(x), xs.lv[ni][l]);
                }
            }
        }
    }
    assert!(clock_won > 0 && flip_won > 0, "both parts decided levels on the thin tape ({clock_won} / {flip_won})");
}

#[test]
fn sn20_the_clock_at_or_after_the_cut_waits_for_the_square_off() {
    // check-build-xc B1 (REQ 70 S14: an own order whose anchor is at or after 15:08:00 is rejected; the position then waits for the
    // square-off; MAIN ruling 14 Nov 15:1x: S14 applies to the combined exit's clock). Deep tapes, so E0 = T.
    // (1) T 15:03:00 (clock E0 + 300 s = 15:08:00, AT the cut) and T 15:04:00 (clock 15:09:00, inside [15:08, 15:10)): the clock is
    //     never sent; with no flip before 15:10:00 (flips in [15:08, 15:10) do not exist) every level is EXACTLY the square-off plan
    //     (Plan::Clock) level by level, with 0 re-walks, and the stored row is the same bytes.
    // (2) T 15:02:58: the clock at 15:07:58 is before the cut: sent, REWALKED, anchor 15:07:58 = the XS plan at that anchor.
    // Mutant m_xc_cut (the cut ignored) sends the 15:08:00 / 15:09:00 clock in (1): caught.
    let sq = at_sod(day(), 15 * 3600 + 10 * 60);
    let origin = at_sod(common_feat::cal::days_from_civil(2018, 1, 1), 0);
    for (case, long) in [(0_u64, true), (1, false), (2, true), (3, false)] {
        let t = if case < 2 {
            let f = move |i: i64| if long { 100_000 + i * 2 } else { 200_000 - i * 2 };
            tape(&f, 500, None)
        } else {
            tape(&seeded(case + 7), 400, None)
        };
        for sod in [15 * 3600 + 3 * 60, 15 * 3600 + 4 * 60] {
            let td = at_sod(day(), sod);
            let xc = price::price(&t, &closest(), td, long, Plan::ClockOrSignal { secs: 300, anchor: sq, immediate: false }, &slots()).unwrap();
            let ck = price::price(&t, &closest(), td, long, Plan::Clock, &slots()).unwrap();
            assert_eq!(xc.entry_first, td, "case {case} T {sod}: E0 = T on a deep tape");
            assert_eq!(xc.lv, ck.lv, "case {case} T {sod}: the clock at or after 15:08:00 is never sent; the 15:10 square-off");
            assert_eq!(xc.rewalks, 0, "case {case} T {sod}: no re-walk");
            for lv in &xc.lv {
                for x in lv.iter().take(5) {
                    assert_eq!(x.exit_anchor, sq);
                    assert_eq!(x.flags & (flag::REWALKED | flag::IMMEDIATE), 0);
                }
            }
            assert_eq!(price::encode(&xc, 5, origin).unwrap(), price::encode(&ck, 5, origin).unwrap());
        }
        let td = at_sod(day(), 15 * 3600 + 2 * 60 + 58);
        let clock = td + 300 * US_S;
        assert_eq!(clock, at_sod(day(), 15 * 3600 + 7 * 60 + 58));
        let xc = price::price(&t, &closest(), td, long, Plan::ClockOrSignal { secs: 300, anchor: sq, immediate: false }, &slots()).unwrap();
        let at = price::price(&t, &closest(), td, long, Plan::Signal { anchor: clock, immediate: false }, &slots()).unwrap();
        assert_eq!(xc.entry_first, td, "case {case}: E0 = T on a deep tape");
        for (ni, lv) in xc.lv.iter().enumerate() {
            for (l, x) in lv.iter().enumerate().take(5) {
                assert_eq!(x.exit_anchor, clock, "case {case} N#{ni} level {l}: the clock at 15:07:58 is before the cut");
                assert_ne!(x.flags & flag::REWALKED, 0, "the clock's win re-walks the entry (S2)");
                assert_eq!(no_rw(*x), at.lv[ni][l], "the XS plan at the clock's anchor, REWALKED aside");
            }
        }
        assert_eq!(xc.rewalks, 15, "one re-walk per level per N (3 N x 5 levels)");
    }
}
