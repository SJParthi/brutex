//! DIALS BUILD on the REAL data (the frozen store's Zerodha candles, the session table, the official bhav, the CA book, the store's
//! GDFL FNO chain census): the seven dials' wiring (value counts = the dials file's radix, OFF = binary k's `live_not_wired`), the
//! plan's vectors (SN-X09 B5 "G-52H on 2019-09-18 is unknown, on 2019-09-19 ready"; the expiry holiday shifts; the month ends) and
//! K11 on the real tables: a datum that arrives after S (the decision session's own official row, later sessions) never moves a
//! reading; a datum in the window moves it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use std::path::{Path, PathBuf};

use common_feat::cal::{at, parse_ymd};
use feat_io::host::Ctx;
use feat_io::{defaults, load_ctx, minute_stream};
use sniper_host::cands::{self, Family};
use sniper_host::ctxdials::{self, IDS, WEEKS_52};
use sniper_host::esn1::{Inputs, load_inputs};

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

fn ctx() -> Ctx {
    load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap()
}

fn family(c: &Ctx, inp: &Inputs, tf: &str) -> (Family, feat_io::host::Stream) {
    let store = Path::new(defaults::STORE);
    let (stream, _) = minute_stream(store, "ADANIENT", tf, c);
    let (s1, _) = minute_stream(store, "ADANIENT", "1min", c);
    let facts = feat_io::host::day_facts(&s1, c);
    let lv = feat_io::host::levels(c, &facts);
    let cut = inp.run.hms("last_decision_before_ist").unwrap();
    (cands::build(c, &stream, &lv, (&inp.core.0, &inp.core.1), cut).unwrap(), stream)
}

fn idx(c: &Ctx, ymd: &str) -> usize {
    c.index_of(parse_ymd(ymd).unwrap()).unwrap_or_else(|| panic!("{ymd} is not a session"))
}

#[test]
fn sn_x09_b5_g52_is_unknown_on_2019_09_18_and_ready_on_2019_09_19() {
    let c = ctx();
    assert_eq!(ctxdials::hl52(&c, idx(&c, "2019-09-18")), None, "G-52H on 2019-09-18 is unknown (its window reaches before the CONT start)");
    let (h, l) = ctxdials::hl52(&c, idx(&c, "2019-09-19")).expect("G-52H on 2019-09-19 is ready");
    assert!(h > l && l > 0, "52-week high {h} low {l}");
}

#[test]
fn k11_g52_never_reads_the_decision_session_or_later_and_reads_its_window() {
    let mut c = ctx();
    let i = idx(&c, "2024-03-05");
    let base = ctxdials::hl52(&c, i).unwrap();
    // the decision session's own official row (published after its close) and every later row: no effect.
    let later: Vec<i32> = c.sessions.iter().skip(i).map(|s| s.day).collect();
    for d in &later {
        if let Some(b) = c.bhav.get_mut(d) {
            b.high *= 3;
            b.low /= 3;
        }
    }
    assert_eq!(ctxdials::hl52(&c, i), Some(base), "no datum of session t or later moves G-52H / G-52L at t");
    // a session inside the 252-session window: the reading moves.
    let w = c.sessions[i - 10].day;
    c.bhav.get_mut(&w).unwrap().high = base.0 * 2;
    assert_eq!(ctxdials::hl52(&c, i).map(|x| x.0), Some(base.0 * 2));
    // the session just beyond the window (the 253rd session before t) is not read.
    let mut c2 = ctx();
    let mut n = 0;
    let mut j = i;
    while n < WEEKS_52 + 1 {
        j -= 1;
        n += 1;
    }
    let beyond = c2.sessions[j].day;
    c2.bhav.get_mut(&beyond).unwrap().high = base.0 * 5;
    assert_eq!(ctxdials::hl52(&c2, i), Some(base), "the 253rd session before t is outside the window");
}

#[test]
fn k11_daily_atr_never_reads_the_decision_session_or_later() {
    let mut c = ctx();
    let i = idx(&c, "2024-03-05");
    let (base, holes) = ctxdials::daily_atr(&c);
    let a = base[i].expect("daily ATR ready on 2024-03-05");
    // the official CM file (scout/bhav-cm-adanient.tsv) ends 2026-09-30: the 20 October 2026 sessions are the only holes.
    let named: Vec<String> = holes.iter().map(|d| common_feat::cal::ymd(*d)).collect();
    assert_eq!(named.len(), 20, "{named:?}");
    assert!(named.iter().all(|d| d.starts_with("2026-10-")), "{named:?}");
    let later: Vec<i32> = c.sessions.iter().skip(i).map(|s| s.day).collect();
    for d in &later {
        if let Some(b) = c.bhav.get_mut(d) {
            b.high *= 2;
        }
    }
    assert_eq!(ctxdials::daily_atr(&c).0[i], Some(a), "session t's own row and later rows never move the ATR known at t");
    let p = c.sessions[i - 1].day;
    c.bhav.get_mut(&p).unwrap().high *= 2;
    assert_ne!(ctxdials::daily_atr(&c).0[i], Some(a), "the previous session's row moves it");
    // unready right after the CONT start and on the rights ex-date (K13 X3 reset), ready again after `need` sessions.
    assert_eq!(base[idx(&c, "2018-10-01")], None);
    assert_eq!(base[idx(&c, "2025-11-17")], None, "the 2025-11-17 rights ex-date resets the CONT reading");
}

#[test]
fn d_exp_and_d_mend_on_the_real_calendars() {
    let c = ctx();
    let (t, _) = sniper_host::rows::read_with_sha(ctxdials::EXPIRIES).unwrap();
    let ex = ctxdials::parse_expiries(&t).unwrap();
    assert!(ex.len() >= 100, "{} expiries", ex.len());
    for (d, want) in [("2023-01-25", true), ("2026-03-30", true), ("2025-09-30", true), ("2025-08-28", true), ("2024-03-28", true), ("2024-03-27", false), ("2023-01-24", false)] {
        assert_eq!(ctxdials::expiry_flag(parse_ymd(d).unwrap(), &ex), Some(want), "D-EXP on {d}");
    }
    let days: Vec<i32> = c.sessions.iter().map(|s| s.day).collect();
    let m = ctxdials::month_end_flags(&days, &[]);
    for (d, want) in [("2024-03-28", Some(true)), ("2026-03-30", Some(true)), ("2026-10-30", Some(true)), ("2024-03-27", Some(false)), ("2019-09-30", Some(true))] {
        assert_eq!(m[idx(&c, d)], want, "D-MEND on {d}");
    }
}

#[test]
fn the_seven_dials_wire_with_the_dials_file_radix_and_off_is_binary_k() {
    let inp = load_inputs(&data()).unwrap();
    let c = ctx();
    let (mut fam, stream) = family(&c, &inp, "60min");
    let all: Vec<usize> = (0..fam.cands.len()).collect();
    // OFF (no readings attached): exactly host v1's wiring, the seven named live_not_wired.
    let (v_off, w_off) = cands::outer_values(&fam, &all, true, &inp.dials, None);
    // (the H-* dials are not wired here only because this test attaches no higher timeframe: `attach_htf` is the run's.)
    let nw: Vec<String> = w_off.not_wired.iter().filter(|x| !x.starts_with("H-")).cloned().collect();
    assert_eq!(nw, IDS.iter().map(|s| (*s).to_string()).collect::<Vec<_>>());
    // ON.
    let (lines, _) = ctxdials::attach(&mut fam, &c, &stream, Path::new(defaults::STORE)).unwrap();
    assert_eq!(fam.ctxr.len(), fam.cands.len());
    for long in [true, false] {
        let (v_on, w_on) = cands::outer_values(&fam, &all, long, &inp.dials, None);
        assert!(w_on.not_wired.iter().all(|x| x.starts_with("H-")), "{:?}", w_on.not_wired);
        for id in IDS {
            let di = inp.dials.iter().position(|d| d.id == id).unwrap();
            let n = v_on.iter().filter(|v| v.dial == di).count() as u64;
            assert_eq!(n + 1, w_on.radix[di], "{id}: values + OFF = the file's radix");
            let vals: Vec<u64> = v_on.iter().filter(|v| v.dial == di).map(|v| v.value).collect();
            assert_eq!(vals, (1..=n).collect::<Vec<_>>(), "{id}: value indexes 1..n in the file's order");
            assert!(v_on.iter().filter(|v| v.dial == di).any(|v| v.kept.count() > 0), "{id}: some value keeps a row ({long})");
        }
        // every other dial's values are unchanged by the wiring (same labels, same kept rows).
        let other_on: Vec<(usize, u64, String, Vec<usize>)> =
            v_on.iter().filter(|v| !IDS.contains(&inp.dials[v.dial].id.as_str())).map(|v| (v.dial, v.value, v.label.clone(), v.kept.ones().collect())).collect();
        if long {
            let off: Vec<(usize, u64, String, Vec<usize>)> = v_off.iter().map(|v| (v.dial, v.value, v.label.clone(), v.kept.ones().collect())).collect();
            assert_eq!(other_on, off);
        }
    }
    let joined = lines.join("\n");
    assert!(joined.contains("context dials ON"), "{joined}");
    // the first candidate with G-52H known is on 2019-09-19 or later (SN-X09 B5).
    let first = fam.cands.iter().zip(&fam.ctxr).find(|(_, r)| r.h52.is_some()).map(|(c, _)| c.day).unwrap();
    assert!(first >= parse_ymd("2019-09-19").unwrap(), "{}", common_feat::cal::ymd(first));
}

#[test]
fn x_nifty_and_x_vix_units_on_the_real_store() {
    let inp = load_inputs(&data()).unwrap();
    let c = ctx();
    let (mut fam, stream) = family(&c, &inp, "60min");
    ctxdials::attach(&mut fam, &c, &stream, Path::new(defaults::STORE)).unwrap();
    // 2020-03-23: NIFTY 50 fell about 13% and INDIA VIX stood above 60 (the COVID crash); 2024-03-05: VIX about 15.
    let at_day = |d: &str| fam.cands.iter().zip(&fam.ctxr).find(|(c, _)| c.day == parse_ymd(d).unwrap()).map(|(c, r)| (c.end, *r)).unwrap();
    let (s, r) = at_day("2020-03-23");
    let nb = r.nifty_bp.map(|(n, d)| n / d).unwrap();
    assert!(nb < -500, "X-NIFTY on 2020-03-23 at {s}: {nb} bp");
    assert!(r.vix.unwrap() > 5_000, "X-VIX x100 on 2020-03-23: {:?}", r.vix);
    let (s2, r2) = at_day("2024-03-05");
    let v2 = r2.vix.unwrap();
    assert!((1_000..2_500).contains(&v2), "X-VIX x100 on 2024-03-05: {v2}");
    // K11: the reading at the first 60-minute decision (END 10:15) equals the last completed NIFTY minute ending by 10:15.
    let (bars, _) = feat_io::zstore::read_range(Path::new(defaults::STORE), brutex_core::vendor::Vendor::Zerodha, "INDEX", "INDIAVIX", feat_io::zstore::tf_of("1min").unwrap(), (2024, 3), (2024, 3));
    let d = parse_ymd("2024-03-05").unwrap();
    let want = bars.iter().rev().find(|b| b.end <= s2 && common_feat::cal::ist_day(b.start) == d).map(|b| b.close);
    assert!(s2 >= at(d, 10, 15, 0), "the first 60-minute decision ends at or after 10:15");
    assert_eq!(Some(v2), want);
}
