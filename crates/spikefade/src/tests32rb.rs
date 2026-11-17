//! v3.2 round B (MAIN-RULINGS-ENGINE-v22 addenda 17 + 18 on top of repair A): named tests of the round's engine changes that live outside
//! `split18.rs` / `side17.rs` (the other NSE indices' cubes merged from a second root, their margin rows appended per snapshot).

use crate::cal::NM;
use crate::cube::{Contract, Cube, Sec, SecTape, Und, merge, merge_sec};

/// A small cube of one day: `unds` underlyings (index kind), two contracts each, every array filled with values that name their slot.
fn cube_of(ymd: u32, names: &[&str], sid0: u32, tag: i32) -> (Cube, SecTape) {
    let mut c = Cube { ymd, day: crate::cal::day_of_ymd(ymd), ..Cube::default() };
    for (u, n) in names.iter().enumerate() {
        c.unds.push(Und { name: (*n).to_owned(), kind: 0, spot_seg: 0, spot_sid: sid0 + 900 + u32::try_from(u).unwrap_or(0), nearest: ymd, next: 0 });
        c.spot.extend((0..NM).map(|m| tag * 1_000_000 + i32::try_from(u * 10_000 + m).unwrap_or(0)));
        for k in 0..2_u32 {
            let ci = c.cons.len();
            c.cons.push(Contract { sid: sid0 + u32::try_from(ci).unwrap_or(0), und: u16::try_from(u).unwrap_or(0), kind: 0, right: u8::try_from(k).unwrap_or(0), expiry: ymd, lot: 25, strike: 1_000_000 + i64::from(k), symbol: format!("{n} {k}"), ..Contract::default() });
            let v = |m: usize| tag * 1_000_000 + i32::try_from(ci * 1_000 + m).unwrap_or(0);
            c.close.extend((0..NM).map(v));
            c.high.extend((0..NM).map(|m| v(m) + 1));
            c.low.extend((0..NM).map(|m| v(m) - 1));
            c.flags.extend((0..NM).map(|m| u8::try_from(m % 7).unwrap_or(0)));
            c.vol.extend((0..NM).map(|m| u32::try_from(m).unwrap_or(0) + u32::try_from(tag).unwrap_or(0)));
            c.oi.extend((0..NM).map(|m| u32::try_from(2 * m).unwrap_or(0)));
            c.cumvol.extend((0..=NM).map(|m| u64::try_from(m * (ci + 1)).unwrap_or(0)));
            c.cumval.extend((0..=NM).map(|m| u64::try_from(m * (ci + 7)).unwrap_or(0)));
        }
    }
    let mut t = SecTape { off: vec![0], rows: Vec::new(), lim: None };
    for ci in 0..c.cons.len() {
        for s in 0..(ci + 2) {
            t.rows.push(Sec { sod: 33_300 + u32::try_from(s).unwrap_or(0), high: tag * 100 + i32::try_from(ci).unwrap_or(0), low: tag, vol: u32::try_from(s).unwrap_or(0) });
        }
        t.off.push(u64::try_from(t.rows.len()).unwrap_or(0));
    }
    (c, t)
}

/// v3.2 ROUND B, THE NAMED TEST OF `--cube-extra`: merging another root's cube of the same day keeps every main value at its index and
/// reads every extra value at its shifted index (contracts, their underlying's spot, every minute array, the cumulative sums, the seconds
/// tape); another day, a shared underlying name or a shared contract id is refused.
#[test]
fn v32rb_cube_merge_exact() {
    let (main, mt) = cube_of(20_260_105, &["NIFTY", "BANKNIFTY"], 10, 1);
    let (extra, et) = cube_of(20_260_105, &["FINNIFTY", "MIDCPNIFTY", "NIFTYNXT50"], 5_000, 2);
    let mut c = main.clone();
    let mut t = mt.clone();
    assert!(merge(&mut c, extra.clone()).is_ok());
    merge_sec(&mut t, et.clone());
    assert_eq!(c.unds.len(), 5);
    assert_eq!(c.cons.len(), main.cons.len() + extra.cons.len());
    for ci in 0..main.cons.len() {
        for m in [0, 1, 200, NM - 1] {
            assert_eq!(c.close_at(ci, m), main.close_at(ci, m));
        }
        assert_eq!(t.of(ci).len(), mt.of(ci).len());
    }
    for (j, k) in extra.cons.iter().enumerate() {
        let ci = main.cons.len() + j;
        let kc = &c.cons[ci];
        assert_eq!((kc.sid, kc.strike, kc.right), (k.sid, k.strike, k.right));
        assert_eq!(c.unds[usize::from(kc.und)].name, extra.unds[usize::from(k.und)].name, "contract {ci}'s underlying");
        for m in [0, 5, 374.min(NM - 1)] {
            assert_eq!(c.close_at(ci, m), extra.close_at(j, m));
            assert_eq!(c.spot_at(usize::from(kc.und), m), extra.spot_at(usize::from(k.und), m), "contract {ci}'s spot at slot {m}");
            assert_eq!(c.high[ci * NM + m], extra.high[j * NM + m]);
            assert_eq!(c.flags[ci * NM + m], extra.flags[j * NM + m]);
        }
        assert_eq!(c.cumvol[Cube::px(ci, NM)], extra.cumvol[Cube::px(j, NM)]);
        let (a, b) = (t.of(ci), et.of(j));
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(b).all(|(x, y)| x.sod == y.sod && x.high == y.high && x.vol == y.vol), "contract {ci}'s seconds");
    }
    // refusals
    let mut c2 = main.clone();
    let (other_day, _) = cube_of(20_260_106, &["FINNIFTY"], 5_000, 2);
    assert!(merge(&mut c2, other_day).is_err(), "another day");
    let (dup_name, _) = cube_of(20_260_105, &["NIFTY"], 5_000, 2);
    assert!(merge(&mut c2, dup_name).is_err(), "an underlying in both roots");
    let (dup_sid, _) = cube_of(20_260_105, &["FINNIFTY"], 10, 2);
    assert!(merge(&mut c2, dup_sid).is_err(), "a contract id in both roots");
}

/// v3.2 ROUND B, THE NAMED TEST OF `--margin-extra`: the other indices' rows of the same SPAN file appended to the session's table (header
/// dropped) are parsed by the crate's own snapshot parser beside the main rows: both underlyings' contracts resolve, the main rows unchanged.
#[test]
fn v32rb_margin_extra_rows_appended() {
    let head = "date\tund\tseg\texpiry\tright\tstrike\tfile_price\tund_file_price\tspan_unit_short\tscan_unit\tworst_scenario\tsom_unit\telm_rate_pct\telm_class\telm_oth_pct\telm_otm_pct\tlot\tdelta\tseries_price_scan\tfile\tfile_created\tael_file";
    let row = |und: &str, k: &str| format!("2024-05-10\t{und}\tidx\t20240516\tC\t{k}\t205.75\t22000.150000\t122.62\t122.62\t11\t0\t2.00\tIDX\t\t\t\t0.5000\t122.80\tnsccl.20240510.i1\t202405092252\tael_10052024.csv");
    let mut main = format!("{head}\n{}\n{}\n", row("NIFTY", "22000"), row("NIFTY", "22100")).into_bytes();
    let extra = format!("{head}\n{}\n{}\n", row("FINNIFTY", "21000"), row("MIDCPNIFTY", "11000")).into_bytes();
    let only_main = common_fill::margin::Snapshot::parse(&main).unwrap_or_else(|e| panic!("{e:?}"));
    crate::mtable::append_rows(&mut main, &extra);
    let s = common_fill::margin::Snapshot::parse(&main).unwrap_or_else(|e| panic!("{e:?}"));
    let mut probes = 0;
    for (u, k) in [("NIFTY", 2_200_000_i64), ("FINNIFTY", 2_100_000), ("MIDCPNIFTY", 1_100_000)] {
        let id = s.und_id(u).and_then(|x| s.find_id(x, 20_240_516, 0, k, &mut probes));
        assert!(id.is_some(), "{u} {k} resolves after the append");
    }
    let a = only_main.und_id("NIFTY").and_then(|x| only_main.find_id(x, 20_240_516, 0, 2_210_000, &mut probes));
    let b = s.und_id("NIFTY").and_then(|x| s.find_id(x, 20_240_516, 0, 2_210_000, &mut probes));
    assert_eq!(a, b, "the main rows keep their ids");
    assert!(only_main.und_id("FINNIFTY").is_none());
}

/// v3.2 ROUND B, THE NAMED TEST OF THE SPLIT STATE IN A CHECKPOINT (addendum 20.3): a day-major run's checkpoint carries every account
/// summary's split statistics (`sum_line` under SPLITS18) and every split's pick so far (`Sel18::ckpt_lines`); a resumed run restores them
/// exactly (the resumed run's split files equal the uninterrupted run's: proof XCKPT-B on real data).
#[test]
fn v32rb_ckpt_split_state_roundtrip() {
    use crate::split18::{SCal, Sel18, set_cal};
    crate::sw::set(crate::sw::SPLITS18, true);
    crate::sw::set(crate::sw::WAYA18, false);
    let d0 = crate::cal::parse_day("2018-10-01").unwrap_or(0);
    let days: Vec<i64> = (d0..d0 + 31 * 30).filter(|&d| crate::rules::weekday(d) < 5).collect();
    set_cal(SCal::from_days(&days).unwrap_or_default());
    let c = crate::split18::cal().unwrap_or_else(|| panic!("cal"));
    let mut sel = Sel18::new();
    assert!(!sel.splits.is_empty());
    // a few accounts with closes every 3rd session
    for i in 0..6_u32 {
        let mut s = crate::split18::Ser18 { s: crate::split18::SSeries::new(c), m: None };
        let mut real = 0_i64;
        for (k, _) in days.iter().enumerate().filter(|(k, _)| (k + i as usize) % 3 == 0) {
            let n = (i64::from(i) * 7_919 + i64::try_from(k).unwrap_or(0) * 104_729) % 9_000_000 - 2_000_000;
            real += n;
            s.close(u32::try_from(k).unwrap_or(0) * 86_400 + 40_000, real, n, true);
        }
        let mut sm = crate::acct::Sum { trades: 5, net: real, list: i, ..crate::acct::Sum::default() };
        crate::split18::account(Some(&s), &mut sm, &mut sel, true, (0, 0, i, 0, 0));
        // the summary's split statistics survive the checkpoint line
        let back = crate::ckpt::sum_of_line(&crate::ckpt::sum_line(&sm)).unwrap_or_default();
        assert_eq!(back.sp, sm.sp, "account {i}");
        assert!(sm.sp.run > 0);
    }
    let lines = sel.ckpt_lines();
    let mut fresh = Sel18::new();
    assert!(fresh.ckpt_restore(&lines).is_ok());
    assert_eq!(fresh.ckpt_lines(), lines, "the restored picks write the same checkpoint");
    let picks = |s: &Sel18| s.best.iter().map(|v| v.iter().map(|b| b.map(|x| (x.key, x.tie, x.sm.list, x.after.net))).collect::<Vec<_>>()).collect::<Vec<_>>();
    assert_eq!(picks(&fresh), picks(&sel));
    assert!(sel.best.iter().flatten().any(Option::is_some), "the test holds at least one pick");
}
