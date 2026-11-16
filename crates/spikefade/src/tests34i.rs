//! v3.4 integrate:D NAMED TESTS (MAIN-RULINGS-ENGINE-v22 Addendum 30.2: data:F3's dated stock freeze reader, data:F7's bands / protection
//! model, and the shard kit's CAP20 carriage on the v3.4 tree). Each named mutant (Cargo features `m_v34i_*`) must make its test FAIL. The
//! switches are per test thread; every test starts with all off. Tests that read the REAL tables of data:F3 / data:F7 name their files.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_precision_loss)]

use crate::cal::{NM, at_sod, days_from_civil, ymd_of};
use crate::cube::{Contract, Cube, Sec, Und};
use crate::freeze::{F3_HEADER, Freeze, StockDated};
use crate::int34 as i34;
use crate::sw;
use crate::trade::NL;
use common_fill::{Kind, LevelFill, LevelMask, MAXL, Order, ResendLaw, Side, Strict};

const V34: &str = "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/spikefade-20261114/v34";

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
    for s in 0..crate::fix34::COUNT {
        crate::fix34::set(s, false);
    }
    for s in 0..i34::COUNT {
        i34::set(s, false);
    }
}

fn stock_cube(day: i64, names: &[&str]) -> Cube {
    let mut c = Cube { ymd: ymd_of(day), day, ..Cube::default() };
    for (i, u) in names.iter().enumerate() {
        c.unds.push(Und { name: (*u).to_owned(), kind: 1, spot_seg: 1, spot_sid: 1, nearest: 2026_11_24, next: 0 });
        c.cons.push(Contract { sid: u32::try_from(i).unwrap() + 10, und: u16::try_from(i).unwrap(), kind: 1, right: 0, expiry: 2026_11_24, lot: 500, strike: 100_000, first_min: 0, first_open: 1_000, symbol: format!("S{i}"), n_sec: 0 });
    }
    let n = names.len();
    c.vol = vec![0; n * NM];
    c.flags = vec![0; n * NM];
    c.high = vec![0; n * NM];
    c.low = vec![0; n * NM];
    c.close = vec![0; n * NM];
    c
}

fn row(sym: &str, day: &str, units: u64, kind: &str, nse: &str) -> String {
    format!("{sym}\t{day}\t{units}\t{kind}\tNSE_FO_contract_x.csv.gz\t{}\tFAOP\t\t\t\t\t\t\t\t\t{nse}", "a".repeat(64))
}

fn table(rows: &[String]) -> StockDated {
    let text = format!("{F3_HEADER}\n{}\n", rows.join("\n"));
    StockDated::parse(&text, "t.tsv", "s").unwrap()
}

// ------------------------------------------------------------------------------------------------ F3 (data:F3 section 7)

/// data:F3 7.3: the lookup reads the EXACT session; a session between two rows of a stock gets NONE_IN_RANGE, never the earlier row
/// (mutant m_v34i_f3_latest_row fails).
#[test]
fn v34i_f3_exact_session_rows() {
    reset();
    let st = table(&[row("ACC", "2024-02-06", 3_000, "SOURCE", "ACC"), row("ACC", "2024-02-08", 3_100, "SOURCE", "ACC"), row("SBIN", "2024-02-07", 9_000, "SOURCE", "SBIN")]);
    let d6 = days_from_civil(2024, 2, 6);
    let d7 = days_from_civil(2024, 2, 7);
    let d8 = days_from_civil(2024, 2, 8);
    assert_eq!(st.lookup("ACC", d6).unwrap().unwrap().units, 3_000);
    assert_eq!(st.lookup("ACC", d8).unwrap().unwrap().units, 3_100);
    assert_eq!(st.lookup("ACC", d7), Some(None), "2024-02-07 has no ACC row: NONE_IN_RANGE, never 2024-02-06's 3,000");
    assert_eq!(st.lookup("ACC", days_from_civil(2024, 2, 9)), None, "after the table's last session: the earlier sources answer");
    assert_eq!(st.lookup("ACC", days_from_civil(2024, 2, 5)), None, "before its first session too");
}

/// data:F3 7.3 / 7.6: a stock contract-day in range without its row is REFUSED and counted once per distinct contract-day; with the switch
/// off nothing is refused (mutant m_v34i_f3_none_unsliced fails).
#[test]
fn v34i_f3_none_in_range_refused_counted() {
    reset();
    let d7 = days_from_civil(2024, 2, 7);
    let fz = Freeze { stock: Some(table(&[row("ACC", "2024-02-06", 3_000, "SOURCE", "ACC"), row("SBIN", "2024-02-08", 9_000, "SOURCE", "SBIN")])), ..Freeze::default() };
    let c = stock_cube(d7, &["ACC"]);
    assert!(!crate::freeze::f3_refused_in(&fz, &c, 0), "switch off: never refused (build:D-high's reading)");
    i34::set(i34::F3D, true);
    assert!(crate::freeze::f3_refused_in(&fz, &c, 0), "in range, no row: refused (freeze_none_in_range), never 'no limit'");
    assert!(crate::freeze::f3_refused_in(&fz, &c, 0));
    assert_eq!(crate::freeze::f3_counts()[3], 1, "counted ONCE per distinct contract-day");
    assert_eq!(crate::freeze::slice_units_in(&fz, &c, 0), 0, "a direct lookup slices nothing (the event never reaches pricing)");
    reset();
}

/// data:F3 7.6: SOURCE / RULE-BOUND / GAP-BOUND / ALIAS counted per DISTINCT contract-day (two lookups of one contract-day count once); an
/// alias row (col 16 = the session's NSE symbol) is found under the cube's name (mutant m_v34i_f3_alias_ignored fails: the ETERNAL day reads
/// NONE_IN_RANGE).
#[test]
fn v34i_f3_kind_and_alias_counters_distinct_contract_days() {
    reset();
    i34::set(i34::F3D, true);
    let d = days_from_civil(2024, 2, 6);
    let fz = Freeze { stock: Some(table(&[row("ACC", "2024-02-06", 3_000, "SOURCE", "ACC"), row("ETERNAL", "2024-02-06", 80_000, "SOURCE", "ZOMATO"), row("SBIN", "2024-02-06", 9_100, "RULE-BOUND", "SBIN"), row("TCS", "2024-02-06", 1_000, "GAP-BOUND", "TCS")])), ..Freeze::default() };
    let c = stock_cube(d, &["ACC", "ETERNAL", "SBIN", "TCS"]);
    for _ in 0..2 {
        for ci in 0..4 {
            assert!(crate::freeze::slice_units_in(&fz, &c, ci) > 0, "contract {ci} sliced");
        }
    }
    assert_eq!(crate::freeze::slice_units_in(&fz, &c, 0), 3_000, "3,000 shares / lot 500 = 6 whole lots a slice");
    let [src, rb, gb, none, alias] = crate::freeze::f3_counts();
    assert_eq!((src, rb, gb, none, alias), (2, 1, 1, 0, 1), "SOURCE ACC + ETERNAL (ALIAS), RULE-BOUND SBIN, GAP-BOUND TCS, each once");
    assert_eq!(fz.stock.as_ref().unwrap().nse_of.get(&("ETERNAL".to_owned(), d)).map(String::as_str), Some("ZOMATO"), "the session's NSE symbol kept");
    reset();
}

/// data:F3 7.2: the strict parse: 15 fields, units 0, a repeated (name, session) and a bad kind are each refused with the line number
/// (mutant m_v34i_f3_fields_unchecked fails).
#[test]
fn v34i_f3_header_fields_checked() {
    reset();
    let ok = row("ACC", "2024-02-06", 3_000, "SOURCE", "ACC");
    let parse = |rows: &[String]| StockDated::parse(&format!("{F3_HEADER}\n{}\n", rows.join("\n")), "t.tsv", "s");
    assert!(parse(std::slice::from_ref(&ok)).is_ok());
    let short = ok.rsplit_once('\t').unwrap().0.to_owned();
    assert!(parse(&[short]).unwrap_err().contains("15 fields"), "15 fields refused");
    assert!(parse(&[row("ACC", "2024-02-06", 0, "SOURCE", "ACC")]).unwrap_err().contains("positive"), "units 0 refused");
    assert!(parse(&[ok.clone(), row("ACC", "2024-02-06", 3_100, "SOURCE", "ACC")]).unwrap_err().contains("repeated"), "a repeated (name, session) refused");
    assert!(parse(&[row("ACC", "2024-02-06", 3_000, "BOUND", "ACC")]).unwrap_err().contains("col 4"), "a bad kind refused");
    assert!(parse(&[ok.replace(&"a".repeat(64), &"A".repeat(64))]).unwrap_err().contains("col 6"), "an upper-case sha refused");
    assert!(StockDated::parse("index\teffective_from\tfreeze_qty_units\n", "t", "s").is_err(), "the old header is not data:F3's");
}

/// data:F3 6.1 / 7.9: the five example rows of the REAL table (`v34/tables/STOCK-FREEZE-DATED.tsv`, sha256 94462038...): read as data:F3
/// documents them (mutant m_v34i_f3_units_plus_one fails: NSE's MaxTradQty is the limit + 1).
#[test]
fn v34i_f3_real_rows_spot() {
    reset();
    let p = format!("{V34}/tables/STOCK-FREEZE-DATED.tsv");
    let Ok(text) = std::fs::read_to_string(&p) else {
        panic!("{p}: data:F3's table is needed for this named test");
    };
    let st = StockDated::parse(&text, &p, "real").unwrap();
    assert_eq!(st.n, 400_291, "every row");
    let get = |s: &str, y: i64, m: i64, d: i64| st.lookup(s, days_from_civil(y, m, d)).unwrap().unwrap();
    assert_eq!((get("ACC", 2018, 10, 1).units, get("ACC", 2018, 10, 1).kind), (13_440, 1), "ACC 2018-10-01 13440 RULE-BOUND");
    assert_eq!((get("NESTLEIND", 2024, 2, 5).units, get("NESTLEIND", 2024, 2, 5).kind), (8_169, 2), "NESTLEIND 2024-02-05 8169 GAP-BOUND");
    assert_eq!((get("NESTLEIND", 2024, 2, 6).units, get("NESTLEIND", 2024, 2, 6).kind), (12_000, 0), "NESTLEIND 2024-02-06 12000 SOURCE");
    assert_eq!(st.files[get("NESTLEIND", 2024, 2, 6).file as usize], "NSE_FO_contract_05022024.csv.gz");
    assert_eq!((get("RELIANCE", 2025, 7, 1).units, get("RELIANCE", 2025, 7, 1).kind), (15_000, 0), "RELIANCE 2025-07-01 15000 SOURCE");
    let e = get("ETERNAL", 2025, 1, 2);
    assert_eq!((e.units, e.kind, e.alias), (80_000, 0, true), "ETERNAL 2025-01-02 80000 SOURCE, an ALIAS row");
    assert_eq!(st.nse_of.get(&("ETERNAL".to_owned(), days_from_civil(2025, 1, 2))).map(String::as_str), Some("ZOMATO"));
}

/// P0 of F3: with the switch off data:F3's file is read by the OLD branch (columns 1-3, "the latest row on or before the session"), exactly as
/// build:D-high's v3.4 reads it, and no stock table is held.
#[test]
fn v34i_f3_switch_off_p0() {
    reset();
    let dir = std::env::temp_dir().join(format!("sf34i-f3-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("STOCK-FREEZE-DATED.tsv");
    std::fs::write(&p, format!("{F3_HEADER}\n{}\n{}\n", row("ACC", "2024-02-06", 3_000, "SOURCE", "ACC"), row("ACC", "2024-02-08", 3_100, "SOURCE", "ACC"))).unwrap();
    let mut f = Freeze::default();
    f.add(p.to_str().unwrap()).unwrap();
    assert!(f.stock.is_none(), "switch off: the old branch");
    assert_eq!(f.units("ACC", days_from_civil(2024, 2, 7)), Some((3_000, true)), "the old reading: the latest row on or before");
    i34::set(i34::F3D, true);
    let mut g = Freeze::default();
    g.add(p.to_str().unwrap()).unwrap();
    assert!(g.stock.is_some(), "switch on: data:F3's exact reader");
    reset();
}

// ------------------------------------------------------------------------------------------------ CAP20 carried by the checkpoint and the kit

/// integrate:D: the CAP20 sweep counters (accounts kept out, window / split refusals) ride in the checkpoint (CAPC, CAP20 on only) and in the
/// shard kit's batch files (MERGE-SW cap20 + CAPC, summed by the merge), so a resumed or merged run prints the uninterrupted run's counts
/// (mutant m_v34i_cap_counts_not_carried fails: a resume printed the later batches' alone).
#[test]
fn v34i_cap_counts_survive_checkpoint_and_merge() {
    reset();
    let dir = std::env::temp_dir().join(format!("sf34i-cap-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("idx.ckpt");
    let c = crate::ckpt::Ckpt { fingerprint: "fp".to_owned(), next_b0: 3, batches: 1, cap: Some([7, 2, 1, 4]), ..crate::ckpt::Ckpt::default() };
    crate::ckpt::save(&p, &c).unwrap();
    let back = crate::ckpt::load(&p, "fp", 0, 0).unwrap().unwrap();
    assert_eq!(back.cap, Some([7, 2, 1, 4]), "the checkpoint carries the CAP20 sweep counters");
    let c0 = crate::ckpt::Ckpt { fingerprint: "fp".to_owned(), ..crate::ckpt::Ckpt::default() };
    crate::ckpt::save(&p, &c0).unwrap();
    assert!(!std::fs::read_to_string(&p).unwrap().contains("CAPC"), "CAP20 off: no CAPC line (the v3.3 file byte for byte)");
    // the restore adds them to this process's counters
    let s0 = crate::fix34::cap_snapshot(0);
    crate::fix34::cap_restore(0, [3, 1, 0, 2]);
    let s1 = crate::fix34::cap_snapshot(0);
    assert_eq!([s1[0] - s0[0], s1[1] - s0[1], s1[2] - s0[2], s1[3] - s0[3]], [3, 1, 0, 2]);
    // the batch file and the merge
    let mk = |k: usize, capc: [u64; 4]| crate::kit::Bck { fp: "fp".to_owned(), k, nb: 2, p0: k, p1: k + 1, nl: 2, cap20: true, capc: Some(capc), s18: (0, 0), acct: (0, "0".repeat(64)), prov: vec![("source".into(), "S".into())], ..crate::kit::Bck::default() };
    let b0 = mk(0, [1, 0, 0, 1]);
    let txt = b0.text();
    assert!(txt.contains("\tcap20=1") && txt.contains("CAPC\t1\t0\t0\t1"), "the batch file names the switch and its counters");
    let back = crate::kit::Bck::parse(&txt).unwrap();
    assert_eq!((back.cap20, back.capc), (true, Some([1, 0, 0, 1])));
    let (m, _) = crate::ckmerge::merge(vec![back, mk(1, [2, 3, 1, 0])], 2).unwrap();
    assert_eq!(m.cap, Some([3, 3, 1, 1]), "the merge SUMS the batches' counters");
    let mut odd = mk(1, [0; 4]);
    odd.capc = None;
    assert!(crate::ckmerge::merge(vec![mk(0, [1, 0, 0, 1]), odd], 2).is_err(), "a batch under CAP20 without its counters is refused");
    crate::fix34::set(crate::fix34::CAP20, false);
    reset();
}

// ------------------------------------------------------------------------------------------------ F7: the tables (data:F7 section 6)

/// data:F7 `f7_rules_tables_load_dated`: the REAL engine-facing table parses into 14 contiguous segments 2018-10-01 .. 2026-11-13; a dropped
/// row is refused (mutant m_v34i_f7_segment_gap fails); the minimum range switches between 2024-06-07 (Rs 3) and 2024-06-10 (Rs 20).
#[test]
fn v34i_f7_rules_tables_load_dated_and_minrange() {
    reset();
    let p = format!("{V34}/tables/F7-PROTECTION-READINGS.tsv");
    let text = std::fs::read_to_string(&p).unwrap();
    let r = crate::f7::Readings::parse(&text, &p).unwrap();
    assert_eq!(r.segs.len(), 14);
    assert_eq!((r.segs[0].from, r.segs.last().unwrap().to), (days_from_civil(2018, 10, 1), days_from_civil(2026, 11, 13)));
    assert_eq!(r.seg(days_from_civil(2024, 6, 7)).unwrap().min_range, 300, "Rs 3 to the session of 2024-06-07");
    assert_eq!(r.seg(days_from_civil(2024, 6, 10)).unwrap().min_range, 2_000, "Rs 20 from 2024-06-10");
    assert!(!r.seg(days_from_civil(2022, 10, 28)).unwrap().lpp_idx_wm && r.seg(days_from_civil(2022, 10, 31)).unwrap().lpp_idx_wm, "LPP index W/M from 2022-10-31");
    assert!(!r.seg(days_from_civil(2023, 2, 3)).unwrap().lpp_stk && r.seg(days_from_civil(2023, 2, 6)).unwrap().lpp_stk, "LPP stock from 2023-02-06");
    assert!(!r.seg(days_from_civil(2023, 3, 30)).unwrap().lpp_idx_other && r.seg(days_from_civil(2023, 3, 31)).unwrap().lpp_idx_other, "every index option from 2023-03-31");
    assert!(r.seg(days_from_civil(2026, 3, 31)).unwrap().rm && !r.seg(days_from_civil(2026, 4, 1)).unwrap().rm, "RM before 2026-04-01 only");
    // a dropped row leaves a gap
    let lines: Vec<&str> = text.lines().collect();
    let drop_at = lines.iter().position(|l| l.starts_with("2022-05-02")).unwrap();
    let gapped: String = lines.iter().enumerate().filter(|(i, _)| *i != drop_at).map(|(_, l)| format!("{l}\n")).collect();
    assert!(crate::f7::Readings::parse(&gapped, "gap").is_err(), "a missing segment is refused");
}

/// data:F7 `f7_opr_file_is_next_session` + `f7_opr_reader_sha_and_columns`: data:F7's manifest maps session 2026-11-02 to NSE's contract file
/// of 2026-10-30 (the file of D serves the next session) and 2026-11-09 to 08112026 (the Muhurat session's); the reader checks the sha256
/// (mutant m_v34i_f7_sha_unchecked fails) and reads NIFTY 2026-11-09 25000 PE = [135650, 251520] paise; NSE's 1980 epoch 1478701800 is
/// 2026-11-09.
#[test]
fn v34i_f7_opr_file_is_next_session_and_columns() {
    reset();
    let p = format!("{V34}/tables/F7-OPR-SESSIONS.tsv");
    assert!(crate::f7::is_manifest(&p));
    let text = std::fs::read_to_string(&p).unwrap();
    let find = |s: &str| text.lines().find(|l| l.starts_with(s)).unwrap().split('\t').map(str::to_owned).collect::<Vec<_>>();
    let r02 = find("20261102\t");
    assert!(r02[2].ends_with("NSE_FO_contract_30102026.csv.gz"), "session 2026-11-02 <- file 30102026");
    assert!(find("20261109\t")[2].ends_with("NSE_FO_contract_08112026.csv.gz"), "2026-11-09 <- 08112026 (the Muhurat session's file)");
    assert!(find("20261111\t")[2].ends_with("NSE_FO_contract_09112026.csv.gz"), "2026-11-11 <- 09112026 (no session on 10 Nov)");
    assert_eq!(crate::f7::ymd_of_nse(1_478_701_800), 2026_11_09);
    assert!(crate::f7::load_file_day(&r02[2], &"0".repeat(64)).unwrap_err().contains("sha256"), "a wrong sha256 is refused");
    let db = crate::f7::load_file_day(&r02[2], &r02[4]).unwrap();
    assert!(db.exact);
    let b = db.by["NIFTY"][&(2026_11_09, 2_500_000, 1)];
    assert_eq!((b.0, b.1), (135_650, 251_520), "NIFTY 2026-11-09 25000 PE on 2026-11-02");
}

/// data:F7 4.4 / 5.1: the no-file bound from the PREVIOUS session's SETTLE (never the close: mutant m_v34i_bound_uses_close fails): lower
/// = max(5, settle - min range), upper = settle + min range, tick 5, not exact; a strike-adjusted (symbol, session) has no bound.
#[test]
fn v34i_f7_nofile_bound_from_settle() {
    reset();
    let d = days_from_civil(2019, 3, 5);
    let text = "INSTRUMENT,SYMBOL,EXPIRY_DT,STRIKE_PR,OPTION_TYP,OPEN,HIGH,LOW,CLOSE,SETTLE_PR,CONTRACTS,VAL_INLAKH,OPEN_INT,CHG_IN_OI,TIMESTAMP,\nOPTIDX,NIFTY,07-Mar-2019,11000,CE,10,12,9,11.5,10.25,1,1,1,1,04-MAR-2019,\nOPTSTK,INFY,28-Mar-2019,700,PE,2,2,1,1.2,0.04,1,1,1,1,04-MAR-2019,\nFUTIDX,NIFTY,28-Mar-2019,0,XX,1,1,1,1,1,1,1,1,1,04-MAR-2019,\nOPTSTK,TCS,28-Mar-2019,2000,CE,2,2,1,1.2,1.5,1,1,1,1,04-MAR-2019,\n";
    let ca: std::collections::HashSet<(String, i64)> = [("TCS".to_owned(), d)].into_iter().collect();
    let b = crate::f7::parse_bound(text, d, 300, Some(&ca), "fo04MAR2019bhav.csv.zip").unwrap();
    assert!(!b.exact);
    assert_eq!(b.by["NIFTY"][&(2019_03_07, 1_100_000, 0)], (725, 1_325, 5), "settle Rs 10.25: [Rs 7.25, Rs 13.25]");
    assert_eq!(b.by["INFY"][&(2019_03_28, 70_000, 1)], (5, 304, 5), "settle Rs 0.04: the lower floored at 5 paise");
    assert!(!b.by.contains_key("TCS"), "a strike-adjusted symbol on its ex-session has no bound");
}

// ------------------------------------------------------------------------------------------------ F7: the strict readings (crate v2.6 S1)

fn secs(rows: &[(i64, i64, i64, u32)]) -> Vec<Sec> {
    rows.iter().map(|&(s, h, l, v)| Sec { sod: u32::try_from(s).unwrap(), high: i32::try_from(h).unwrap(), low: i32::try_from(l).unwrap(), vol: v }).collect()
}

fn walk_strict(dt: &crate::trade::DayTape, s: Option<Strict>, side: Side, anchor: i64, stop: i64, lim: i64, kind: Kind, want: u64) -> LevelFill {
    let dl = crate::trade::dial().unwrap();
    let tape = dt.tape().unwrap().with_strict(s);
    let d = dt_day(dt);
    let o = Order { side, anchor: at_sod(d, anchor), stop: at_sod(d, stop), unit: 50, limit: Some(lim), kind };
    let mut w = [want; MAXL];
    let mut out = [LevelFill::default(); MAXL];
    common_fill::walk(&tape, &o, &dl, LevelMask::all(NL), &mut w, None, None, &mut out, None).unwrap();
    out[4]
}

fn dt_day(_dt: &crate::trade::DayTape) -> i64 {
    days_from_civil(2026, 11, 4)
}

fn strict(open_sod: i64, up: Option<i64>, dn: Option<i64>, exact: bool, lpp: bool, law: ResendLaw) -> Strict {
    Strict { open: at_sod(days_from_civil(2026, 11, 4), open_sod), up, dn, exact, lpp, law }
}

const P10: ResendLaw = ResendLaw::Protection { bp: 1_000, tick: 5 };

/// data:F7 `xm_band_lock_zero_share` (F-H2 / X10): one-price seconds AT the known upper band give 0 usable buy volume; the buy fills from the
/// first second with a print below it (mutant m_v34i_band_lock_off fails: it fills at the band at once).
#[test]
fn v34i_xm_band_lock_zero_share() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let mut r = vec![(t - 1, 14_000, 14_000, 10_000)];
    for s in 0..5 {
        r.push((t + s, 15_000, 15_000, 10_000));
    }
    for s in 5..20 {
        r.push((t + s, 14_900, 14_800, 10_000));
    }
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let st = strict(t - 1, Some(15_000), Some(1_000), true, false, P10);
    let f = walk_strict(&dt, Some(st), Side::Buy, t, t + 60, 16_500, Kind::SignalExit, 500);
    let fb = f.first.expect("it fills below the band");
    assert!(fb.block.at >= at_sod(d, t + 5), "nothing while every print sat at the band");
    assert_eq!(fb.px, 14_900);
    assert!(f.locked_secs >= 5, "the locked seconds counted: {}", f.locked_secs);
    let plain = walk_strict(&dt, None, Side::Buy, t, t + 60, 16_500, Kind::SignalExit, 500);
    assert_eq!(plain.first.map(|b| b.px), Some(15_000), "no strict view: AT the band at once (v3.3 / R0)");
}

/// data:F7 `f7_band_lock_mixed_second_keeps_volume`: a second printing at the band AND below keeps its volume (only one-price seconds at the
/// band are locked; mutant m_v34i_lock_any_print_at_band fails).
#[test]
fn v34i_f7_band_lock_mixed_second_keeps_volume() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let r = vec![(t - 1, 14_000, 14_000, 10_000), (t, 15_000, 14_950, 10_000), (t + 1, 14_900, 14_900, 10_000)];
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let st = strict(t - 1, Some(15_000), Some(1_000), true, false, P10);
    let f = walk_strict(&dt, Some(st), Side::Buy, t, t + 10, 16_500, Kind::SignalExit, 500);
    assert_eq!(f.first.map(|b| b.block.at), Some(at_sod(d, t)), "the mixed second fills at once (at its worst print, the band)");
    assert_eq!(f.locked_secs, 0);
}

/// data:F7 `f7_band_after_flex_known_level`: after a print above the file band (a flex), the known level is the session's highest print so
/// far: a one-price second at a NEW high is locked, a one-price second below that high keeps its volume.
#[test]
fn v34i_f7_band_after_flex_known_level() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    // the band 15,000 flexed earlier: a print at 16,000 before t
    let r = vec![(t - 100, 16_000, 15_500, 1_000), (t - 1, 15_800, 15_800, 1_000), (t, 16_500, 16_500, 10_000), (t + 1, 15_900, 15_900, 10_000)];
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let st = strict(t - 200, Some(15_000), Some(1_000), true, false, ResendLaw::Protection { bp: 1_000, tick: 5 });
    let f = walk_strict(&dt, Some(st), Side::Buy, t, t + 10, 20_000, Kind::SignalExit, 50);
    assert_eq!(f.first.map(|b| (b.block.at, b.px)), Some((at_sod(d, t + 1), 15_900)), "16,500 = a new high, one price: locked; 15,900 below 16,000: fills");
    let (lvl, unflexed) = dt.tape().unwrap().known_band(&st, at_sod(d, t), Side::Buy);
    assert_eq!((lvl, unflexed), (Some(16_000), false), "the known level = the highest print so far; flexed: no clip");
}

/// data:F7 `f7_band_clip_only_when_known`: a limit above an EXACT unflexed band is clipped to it (counted; no fill changes); never to a bound
/// (exact = false).
#[test]
fn v34i_f7_band_clip_only_when_known() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let r = vec![(t - 1, 14_000, 14_000, 10_000), (t, 14_900, 14_800, 10_000)];
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let ex = walk_strict(&dt, Some(strict(t - 1, Some(15_000), Some(1_000), true, false, P10)), Side::Buy, t, t + 5, 16_500, Kind::SignalExit, 50);
    assert!(ex.band_clipped >= 1, "14,000 + 10% = 15,400 > 15,000: clipped, counted");
    assert_eq!(ex.first.map(|b| b.px), Some(14_900), "the fill unchanged");
    let bd = walk_strict(&dt, Some(strict(t - 1, Some(15_000), Some(1_000), false, false, P10)), Side::Buy, t, t + 5, 16_500, Kind::SignalExit, 50);
    assert_eq!(bd.band_clipped, 0, "a bound never clips a limit");
}

/// data:F7 `xm_resend_limit_inside_nse_lpp_band` (X05): a re-send whose limit is above High LPP (the lowest possible 30-second reference
/// + Rs 20 / x 1.4) gets 0 capacity at that send (counted); once the reference catches up the re-send fills (mutant m_v34i_lpp_ignored
/// fails: the rejected block fills).
#[test]
fn v34i_xm_resend_limit_inside_nse_lpp_band() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    // the reference window holds a print at Rs 100 (High LPP_lb = Rs 140); the last print before t is Rs 135 (limit Rs 148.50 > 140)
    let mut r = vec![(t - 40, 10_000, 10_000, 1_000)];
    for s in (t - 39)..t {
        r.push((s, 13_500, 13_500, 100));
    }
    for s in t..(t + 60) {
        r.push((s, 14_000, 13_900, 10_000));
    }
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let on = walk_strict(&dt, Some(strict(t - 100, None, None, false, true, P10)), Side::Buy, t, t + 60, 20_000, Kind::SignalExit, 50);
    assert!(on.lpp_rejected >= 1, "the first re-sends are rejected: {}", on.lpp_rejected);
    let fb = on.first.expect("filled once the reference caught up");
    assert!(fb.block.at > at_sod(d, t), "not in the rejected first block");
    let off = walk_strict(&dt, Some(strict(t - 100, None, None, false, false, P10)), Side::Buy, t, t + 60, 20_000, Kind::SignalExit, 50);
    assert_eq!(off.first.map(|b| b.block.at), Some(at_sod(d, t)), "LPP not live for the class: the first block fills");
}

/// data:F7 `f7_lpp_ref_bounds_bracket_every_phase` (the engine side): a buy's band is read from the LOWEST possible reference, a sale's from
/// the HIGHEST (the strict side of the bracket); mutant m_v34i_lpp_ref_ub_for_buy fails (a buy read from the highest is kinder).
#[test]
fn v34i_f7_lpp_ref_bounds_for_buy() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let r = vec![(t - 40, 10_000, 10_000, 1_000), (t - 35, 20_000, 20_000, 1_000)];
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let tape = dt.tape().unwrap();
    // reference in [Rs 100, Rs 200]: a buy at Rs 150 is above Rs 100 x 1.4 = Rs 140 (rejected); under the highest reference Rs 280 (admitted)
    assert_eq!(tape.lpp_check(at_sod(d, t), Side::Buy, 15_000), common_fill::LppCheck::Reject);
    assert_eq!(tape.lpp_check(at_sod(d, t), Side::Sell, 11_000), common_fill::LppCheck::Reject, "a sale below Rs 200 x 0.6 = Rs 120? no: 110 < 120: rejected");
    assert_eq!(tape.lpp_check(at_sod(d, t), Side::Sell, 13_000), common_fill::LppCheck::Admit);
    assert_eq!(tape.lpp_check(at_sod(d, t + 20 * 60), Side::Buy, 15_000), common_fill::LppCheck::Unknown, "no print within 15 min 30 s: unknown");
}

/// data:F7 `f7_mpp_nse_cap_and_lpp_clip`: NSE's market-order cap LTP + max(20%, Rs 10): LTP Rs 30 -> a buy fills up to Rs 40 (mutant
/// m_v34i_mpp_no_min_abs fails: Rs 36 refuses the Rs 39 block).
#[test]
fn v34i_f7_mpp_nse_cap() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    // two prices in the second (a one-price second at a new high would be band-locked: no band known = the running high)
    let r = vec![(t - 1, 3_000, 3_000, 1_000), (t, 3_900, 3_850, 10_000)];
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let f = walk_strict(&dt, Some(strict(t - 1, None, None, false, false, ResendLaw::NseMarket { lpp_clip: false })), Side::Buy, t, t + 1, 3_300, Kind::SignalExit, 50);
    assert_eq!(f.first.map(|b| b.px), Some(3_900), "Rs 39 inside the Rs 40 cap");
}

// ------------------------------------------------------------------------------------------------ F7: the sale and the booking (trade.rs)

fn ctx(rd: [bool; 4], lpp: bool) -> crate::f7::Ctx {
    crate::f7::Ctx { band: None, lpp, mpp_lpp: false, mpp_nse: true, tick: 5, rd, open: at_sod(days_from_civil(2026, 11, 4), crate::cal::OPEN_S) }
}

/// data:F7 `f7_entry_strict_r5`: the sale by R5: a 5% protection limit (a block 7% below the reference does not fill; R0's 10% fills it and
/// the level is counted as refused by F7); NSE's LPP rejects a sale far below the reference at T (mutant m_v34i_entry_uses_r10 fails).
#[test]
fn v34i_f7_entry_strict_r5() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    // two prices a second (a one-price second at a new low would be band-locked for the sale: no band known = the running low)
    let r = vec![(t - 1, 10_000, 10_000, 1_000), (t, 9_310, 9_300, 100_000), (t + 2, 9_310, 9_300, 100_000)];
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let dl = crate::trade::dial().unwrap();
    let tape = dt.tape().unwrap();
    let p0 = crate::trade::record_base(&tape, d, t, 50, 500, &dl).ok().unwrap();
    assert!(!p0.fills[4].is_empty(), "R0 (10%): the Rs 93 blocks fill");
    let p5 = crate::trade::record_base_f7(&tape, d, t, 50, 500, &dl, Some(ctx([true, true, true, false], false))).ok().unwrap();
    assert!(p5.fills[4].is_empty(), "R5 (5%: limit Rs 95): nothing at Rs 93");
    assert!(p5.sale_f7 & (1 << 4) != 0, "the level R0 fills and R5 refuses is counted");
}

/// data:F7 `f7_worst_of_readings_books_dearest`: a buy-back during a climb of +8% a second: R10 re-sends follow it and fill early, R5's 5%
/// re-sends stay past the limit until the climb stops (dearer): the booking takes R5 and prints R0 beside; a flat day books R0 (ties go to
/// the earlier reading) (mutant m_v34i_worst_of_cheapest fails).
#[test]
fn v34i_f7_worst_of_readings_books_dearest() {
    reset();
    sw::set(sw::RESEND, true);
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let x = t + 30;
    let mut r = vec![(t - 1, 10_000, 10_000, 1_000)];
    for s in t..x {
        r.push((s, 10_000, 9_900, 100_000));
    }
    let mut px = 10_000_i64;
    for s in x..(x + 10) {
        px = px * 108 / 100;
        // two prices a second: no band lock (a one-price new high would be locked under an unknown band), only the protection limits differ
        r.push((s, px, px - 10, 100_000));
    }
    for s in (x + 10)..(x + 40) {
        r.push((s, px, px - 50, 100_000));
    }
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let dl = crate::trade::dial().unwrap();
    let tape = dt.tape().unwrap();
    let mut p = crate::trade::record_base_f7(&tape, d, t, 50, 500, &dl, Some(ctx([true, true, true, false], false))).ok().unwrap();
    let tr = crate::trade::trip(&mut p, &tape, d, 50, 4, 500, Some(x), &dl).unwrap();
    let f = tr.f7.expect("F7 facts on the trip");
    assert_eq!(f.rd, 2, "R5 booked (the dearest): net {} vs R0 {}", tr.net, f.r0_net);
    assert!(tr.net < f.r0_net, "the booked net below R0's");
    // a flat day: every reading fills alike: R0 booked
    let flat: Vec<(i64, i64, i64, u32)> = (t - 1..x + 40).map(|s| (s, 10_000, 9_900, 100_000)).collect();
    let dt2 = crate::trade::DayTape::new(d, &secs(&flat), 1).unwrap();
    let tape2 = dt2.tape().unwrap();
    let mut p2 = crate::trade::record_base_f7(&tape2, d, t, 50, 500, &dl, Some(ctx([true, true, true, false], false))).ok().unwrap();
    let tr2 = crate::trade::trip(&mut p2, &tape2, d, 50, 4, 500, Some(x), &dl).unwrap();
    assert_eq!(tr2.f7.unwrap().rd, 0, "a day with no F7 event books R0");
    reset();
}

/// data:F7 `f7_resend_cut_1523`: the strict readings send nothing at or after 15:23:00: a square-off whose prints come only after 15:23 is
/// left open (valued per ruling item 3, Dhan's fee) while R0 fills it (mutant m_v34i_cut_1523_off fails).
#[test]
fn v34i_f7_resend_cut_1523() {
    reset();
    sw::set(sw::END_1530, true);
    i34::set(i34::PROT, true);
    i34::set(i34::CUT, true);
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let sq = crate::cal::SQ_S;
    // the sale must FILL (two prices: a one-price second at the running low would be lower-band-locked for the strict sale)
    let mut r = vec![(t - 1, 10_000, 10_000, 1_000), (t, 10_000, 9_990, 100_000)];
    r.push((sq - 1, 10_000, 10_000, 1_000));
    for s in (crate::trade::CUT_1523_S + 5)..(crate::trade::CUT_1523_S + 60) {
        // two prices a second (a one-price second at the running high would be band-locked: the cut, not the lock, is under test)
        r.push((s, 10_000, 9_990, 100_000));
    }
    let dt = crate::trade::DayTape::new(d, &secs(&r), 1).unwrap();
    let dl = crate::trade::dial().unwrap();
    let tape = dt.tape().unwrap();
    let mut p = crate::trade::record_base_f7(&tape, d, t, 50, 500, &dl, Some(ctx([true, true, false, false], false))).ok().unwrap();
    let r0 = crate::trade::trip_rd(&mut p, &tape, d, 50, 4, 500, None, &dl, 0).unwrap();
    assert!(r0.units > 0, "the sale filled (the test is not vacuous)");
    assert_eq!(r0.unclosed, 0, "R0's square-off fills after 15:23");
    let r10 = crate::trade::trip_rd(&mut p, &tape, d, 50, 4, 500, None, &dl, 1).unwrap();
    assert_eq!(r10.unclosed, r10.units, "R10 sends nothing at or after 15:23:00: every unit left open (valued)");
    reset();
}

/// P0 of F7: with PROT off the paths carry no F7 context and `trip` is build:D-high's (the same net, no F7 facts).
#[test]
fn v34i_f7_switches_off_byte_identical() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    let flat: Vec<(i64, i64, i64, u32)> = (t - 1..t + 100).map(|s| (s, 10_000, 9_900, 100_000)).collect();
    let dt = crate::trade::DayTape::new(d, &secs(&flat), 1).unwrap();
    let dl = crate::trade::dial().unwrap();
    let tape = dt.tape().unwrap();
    let mut a = crate::trade::record_base(&tape, d, t, 50, 500, &dl).ok().unwrap();
    let mut b = crate::trade::record_base_f7(&tape, d, t, 50, 500, &dl, None).ok().unwrap();
    assert!(a.f7.is_none() && b.f7.is_none());
    let ta = crate::trade::trip(&mut a, &tape, d, 50, 4, 500, Some(t + 30), &dl).unwrap();
    let tb = crate::trade::trip(&mut b, &tape, d, 50, 4, 500, Some(t + 30), &dl).unwrap();
    assert_eq!((ta.net, ta.bb_value, ta.f7), (tb.net, tb.bb_value, None));
    assert!(crate::f7::ctx_of(&Cube::default(), 0).unwrap().is_none(), "PROT off: no context");
}
