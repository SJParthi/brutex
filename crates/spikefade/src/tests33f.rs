//! v3.3 INTEGRATE NAMED TESTS (integrate:v33, MAIN-RULINGS-ENGINE-v22 Addendum 28.2): one per BLOCKING item of the re-checks of round B
//! (`v32/check-engine-1r.md` M1r / M2r / M3r, `v32/check-engine-2r.md` M1 / M2 / M3, matrix X06). Every case is synthetic and integer; each
//! named mutant (Cargo features `m_v33f_*`) must make its test FAIL. The switches are per test thread; every test starts with all off.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_precision_loss)]

use crate::acct::{Sum, Tops};
use crate::cal::{NM, SQ_S, at_sod, day_of_ymd, days_from_civil};
use crate::cube::{Contract, Cube, Sec, Und};
use crate::fix33;
use crate::sw;
use crate::trade::{ACCOUNT, DayTape, dial, record_base, trip};

fn reset() {
    for s in 0..sw::COUNT {
        sw::set(s, false);
    }
    for s in 0..crate::sw33::COUNT {
        crate::sw33::set(s, false);
    }
    for s in 0..fix33::COUNT {
        fix33::set(s, false);
    }
}

/// An index cube of one day: NIFTY, four contracts of lot 25 on the nearest expiry, every minute present, volumes `f` x (contract + 1) x a
/// minute pattern, rising closes (so windows have gains), the cumulative sums from those arrays.
fn cube_lv(ymd: u32, f: u64) -> Cube {
    let exp = ymd + 2;
    let mut c = Cube { ymd, day: day_of_ymd(ymd), ..Cube::default() };
    c.unds.push(Und { name: "NIFTY".to_owned(), kind: 0, spot_seg: 0, spot_sid: 1, nearest: exp, next: 0 });
    c.spot = vec![2_400_000; NM];
    for ci in 0..4_u32 {
        c.cons.push(Contract { sid: 100 + ci, und: 0, kind: 0, right: u8::try_from(ci % 2).unwrap(), expiry: exp, lot: 25, strike: 2_400_000 + i64::from(ci) * 5_000, first_min: 0, first_open: 10_000, symbol: format!("NIFTY-X-{ci}"), n_sec: 0 });
        let v = |m: usize| (u64::from(ci) + 1) * f * 25 * (1 + u64::try_from(m % 5).unwrap());
        let cl = |m: usize| 10_000 + i32::try_from(m).unwrap() * (i32::try_from(ci).unwrap() + 1);
        c.close.extend((0..NM).map(cl));
        c.high.extend((0..NM).map(|m| cl(m) + 5));
        c.low.extend((0..NM).map(|m| cl(m) - 5));
        c.flags.extend((0..NM).map(|_| 5_u8));
        c.vol.extend((0..NM).map(|m| u32::try_from(v(m)).unwrap()));
        c.oi.extend((0..NM).map(|_| 0_u32));
        let (mut cv, mut cval) = (0_u64, 0_u64);
        c.cumvol.push(0);
        c.cumval.push(0);
        for m in 0..NM {
            cv += v(m);
            cval += v(m) * u64::try_from(cl(m)).unwrap();
            c.cumvol.push(cv);
            c.cumval.push(cval);
        }
    }
    c
}

fn lists_of(c: &Cube, di: u16, lv: &crate::screen::Levels) -> Vec<Vec<(u32, i64, u64)>> {
    crate::screen::generate(c, di, 0, lv, false, true, None).lists.iter().map(|l| l.iter().map(|e| (e.ue, e.gain_bp, e.lots)).collect()).collect()
}

/// check-engine-2 M1 / check-engine-1 M2r (the screener level look-ahead): under LV_CAUSAL a day's levels, and so its lists, are byte-identical
/// with and without a LATER day loaded (round B's run-wide levels move with it); the run's first session is a warm-up with no event; the
/// causal percentile over several days' sorted samples equals round B's `levels_from` on their concatenation exactly.
#[test]
fn v33f_causal_levels_ignore_later_days() {
    reset();
    fix33::set(fix33::LV_CAUSAL, true);
    let days = [cube_lv(20_260_302, 1), cube_lv(20_260_303, 2), cube_lv(20_260_304, 9)];
    let raw: Vec<fix33::DaySmp> = days.iter().map(|c| crate::report::level_samples(c, 0)).collect();
    let smp: Vec<Option<fix33::DaySmp>> = raw.iter().cloned().map(|s| Some(fix33::sort_samples(s))).collect();
    // exactness against round B's percentile step (the same samples, the same rounding)
    let cat = |k: usize| -> fix33::DaySmp {
        let mut all: fix33::DaySmp = vec![(Vec::new(), Vec::new()); crate::screen::TFS.len()];
        for d in raw.iter().take(k) {
            for (x, (l, v)) in all.iter_mut().zip(d) {
                x.0.extend(l);
                x.1.extend(v);
            }
        }
        all
    };
    let rb2 = crate::report::levels_from(cat(2));
    let rb3 = crate::report::levels_from(cat(3));
    let ours2 = fix33::levels_of(&[smp[0].as_ref().unwrap(), smp[1].as_ref().unwrap()]);
    assert_eq!((ours2.minvol, ours2.minval, ours2.samples), (rb2.minvol, rb2.minval, rb2.samples), "the causal percentile = round B's on the same samples");
    assert_ne!(rb2.minvol, rb3.minvol, "round B's run-wide levels move when a later day is loaded (the look-ahead)");
    // the causal levels of day 2 with and without day 3
    let two = fix33::causal_levels(&smp[..2], 60, 2);
    let three = fix33::causal_levels(&smp[..3], 60, 2);
    assert!(two[0].is_none() && three[0].is_none(), "the first session: warm-up, no level known");
    let (a, b) = (two[1].as_ref().unwrap(), three[1].as_ref().unwrap());
    assert_eq!((a.0.minvol, a.0.minval, a.0.samples, a.1), (b.0.minvol, b.0.minval, b.0.samples, b.1), "day 2's levels do not read day 3");
    let one = fix33::levels_of(&[smp[0].as_ref().unwrap()]);
    assert_eq!(a.0.minvol, one.minvol, "day 2 reads day 1 only");
    assert_eq!(lists_of(&days[1], 1, &a.0), lists_of(&days[1], 1, &b.0), "day 2's lists byte-identical with and without the later day");
    assert!(lists_of(&days[1], 1, &a.0).iter().any(|l| !l.is_empty()), "day 2 lists events");
    assert!(lists_of(&days[0], 0, &fix33::warm_levels()).iter().all(Vec::is_empty), "the warm-up day lists no event");
    // the window: day 3 with W = 1 reads day 2 only
    let w1 = fix33::causal_levels(&smp[..3], 1, 1);
    assert_eq!(w1[2].as_ref().unwrap().0.minvol, fix33::levels_of(&[smp[1].as_ref().unwrap()]).minvol);
    let (dl, (np, nw, short)) = fix33::day_levels(&three, &[true, true, true], 60);
    assert_eq!((dl.len(), np, nw, short), (3, 3, 1, 2));
    reset();
}

fn pm(mark1: u32, units: u64, sold: i64) -> crate::v32::PMark {
    crate::v32::PMark { units, sold, mark1 }
}

/// check-engine-1 M1r (the Rs 20 L cap between sales): the minute-by-minute reading against the ACCOUNT BASE Rs 20 L + min(0, realized): a
/// position whose margin alone is under the flat Rs 20 L but above the base after a realized loss is caught (minutes counted, the largest
/// excess named); an account inside the base passes; the views a pick reads drop a failing account and keep their checked accounts only.
#[test]
fn v33f_cap_minutes_base_and_exclusion() {
    reset();
    fix33::set(fix33::CAP_MIN, true);
    let d = days_from_civil(2026, 11, 4);
    // marks: one contract-day, the option at Rs 100.00 all day (no running loss for a sale at Rs 100.00 x 50 units)
    let marks: Vec<i32> = vec![10_000; NM];
    // trade 1: 09:20-09:40, lost Rs 50,000 (realized from 09:40); trade 2: 10:00-11:00 with Rs 19,60,000 of margin
    let t1 = (at_sod(d, 33_600), at_sod(d, 34_800), 50_000_000_i64, -5_000_000_i64, d, pm(1, 50, 500_000));
    let t2 = (at_sod(d, 36_000), at_sod(d, 39_600), 196_000_000_i64, 1_000_i64, d, pm(1, 50, 500_000));
    let c = fix33::cap_minutes(&[t1, t2], &marks, &[d]);
    assert_eq!(c.minutes, 375);
    assert_eq!(c.above_flat, 0, "never above the flat Rs 20 L");
    assert_eq!(c.above_base, 60, "10:00-11:00 above Rs 20 L - Rs 50,000 = Rs 19,50,000");
    assert_eq!(c.excess, 1_000_000, "Rs 19,60,000 - Rs 19,50,000");
    assert!(!c.ok() && fix33::cap_words(&c).contains("EXCLUDED"));
    // inside the base: Rs 19,40,000
    let t3 = (at_sod(d, 36_000), at_sod(d, 39_600), 194_000_000_i64, 1_000_i64, d, pm(1, 50, 500_000));
    assert!(fix33::cap_minutes(&[t1, t3], &marks, &[d]).ok());
    // a running loss: marks at Rs 120.00 from 10:30 make the open short Rs 1,000 down: Rs 19,99,500 + Rs 1,000 > Rs 20 L
    let mut m2 = marks.clone();
    for x in m2.iter_mut().skip(75) {
        *x = 12_000;
    }
    let t4 = (at_sod(d, 36_000), at_sod(d, 39_600), 199_950_000_i64, 1_000_i64, d, pm(1, 50, 500_000));
    let c4 = fix33::cap_minutes(&[t4], &m2, &[d]);
    assert_eq!((c4.above_flat, c4.above_base), (30, 30), "10:30-11:00: margin + the running loss above the cap");
    // the views: account 2 fails, account 3 lies beyond the first K = 2 (unchecked): view 0 keeps account 1 only
    let s = |list: u32, net: i64| Sum { list, x: 0, n: 0, net, trades: 3, ndays: 1, days_pos: 1, ..Sum::default() };
    let mut tops = vec![Tops::new(10)];
    for a in [s(1, 300), s(2, 200), s(3, 100)] {
        tops[0].offer(a);
    }
    let bad: std::collections::HashSet<(u32, u16, u8)> = [crate::report3::acct_key33(&s(2, 200))].into_iter().collect();
    crate::report3::cap_retain33(&mut tops, &[(0, 0)], 2, &bad);
    assert_eq!(tops[0].best(0).iter().map(|x| x.list).collect::<Vec<_>>(), vec![1], "the failing account left the view; the unchecked one too");
    assert_eq!(tops[0].best(8).len(), 3, "a view no pick reads is untouched");
    reset();
}

/// check-engine-1 M3r (b): the SEBI fee at the DATED 50% from 2020-06-01 to 2021-03-31 (SEBI/LAD-NRO/GN/2020/011, NSE FA44514) under CHG33;
/// the full fee on either side of the window and with the switch off (round B).
#[test]
fn v33f_sebi_half_dated() {
    reset();
    let r = |y: i64, m: i64, dd: i64| crate::charges::rates_on(days_from_civil(y, m, dd)).unwrap().sebi;
    assert_eq!((r(2020, 6, 1), r(2021, 3, 31)), (1_000, 1_000), "round B: the full Rs 10 per crore");
    fix33::set(fix33::CHG33, true);
    assert_eq!((r(2020, 5, 29), r(2020, 6, 1), r(2021, 3, 31), r(2021, 4, 1)), (1_000, 500, 500, 1_000), "50% inside the dated window only");
    assert_eq!(r(2019, 3, 29), 1_500, "before 2019-04-01: Rs 15 per crore");
    reset();
}

/// check-engine-1 M3r (a): every pick names its trades before 2020-07-01, whose stamp duty is the UNVERIFIED conservative 0.02% on both sides,
/// and its trades on the dated SEBI 50% window.
#[test]
fn v33f_stamp_unverified_labelled() {
    reset();
    fix33::set(fix33::CHG33, true);
    let days = [days_from_civil(2019, 12, 2), days_from_civil(2020, 6, 30), days_from_civil(2020, 7, 1), days_from_civil(2021, 4, 5)];
    let w = fix33::charges_words(&days);
    assert!(w.contains("Stamp duty UNVERIFIED on 2 of 4 trades"), "{w}");
    assert!(w.contains("on 2 of 4 trades.") && w.contains("SEBI fee at the DATED 50%"), "{w}");
    assert!(!fix33::charges_words(&days[2..]).contains("UNVERIFIED"), "no unverified stamp from 2020-07-01");
    reset();
}

/// check-engine-2 M2 (words): the split / side-by-side words print the segment's own slot count (index N 15 / 20 under the NGRID grid), never
/// v3.1's 20 / 40; stock keeps 5 / 10 / 20 / 40; with WORDS33 off the words are round B's.
#[test]
fn v33f_split_words_segment_grid() {
    reset();
    sw::set(sw::NGRID, true);
    assert_eq!((fix33::n_words(0, 2), fix33::n_words(0, 3)), (20, 40), "round B's words (v3.1 grid)");
    fix33::set(fix33::WORDS33, true);
    assert_eq!((0..4).map(|i| fix33::n_words(0, i)).collect::<Vec<_>>(), vec![5, 10, 15, 20], "index words = the index grid");
    assert_eq!((0..4).map(|i| fix33::n_words(1, i)).collect::<Vec<_>>(), vec![5, 10, 20, 40], "stock words = the stock grid");
    reset();
}

/// check-engine-2 M2 (label): the vendor of a day comes from its cube manifest's own builder / source lines: a GDFL history day is never
/// labelled Tickvault (the two header forms the cube builders write).
#[test]
fn v33f_vendor_from_manifest() {
    reset();
    let gdfl = "builder\tspikefade-cube v1 / GDFL source gdfl-3 + C17 index-lot fallback (scratch/spikefade-gdfl-full g3)\nsource\tGDFL store 1-s bars (runtime/store/bars/gdfl/NSE/{FNO,INDEX,CASH})\nday\t2022-07-01";
    let tv = "builder\tspikefade-cube v1 (main 5b507203 wf step, 2026-11-14)\ninput\t/x/state/tickvault-import-20261114/stage/instruments/2026-11-02/instruments-2026-11-02.csv.gz\t1\t2\tab";
    assert_eq!(fix33::vendor_of_text(gdfl), "GDFL");
    assert_eq!(fix33::vendor_of_text(tv), "Tickvault");
    assert_eq!(fix33::vendor_of_text("builder\tsomething else"), "unknown");
    reset();
}

/// check-engine-2 M3 (day-major step timing): the day-major stage B's sampler times every stride-th account-day walk with its steps, so a
/// day-major run records sampled blocks with p50 <= p99; the kept set stays bounded (the stride doubles); zero blocks print NOT MEASURED.
#[test]
fn v33f_day_major_step_sampled() {
    reset();
    assert!(!fix33::StepSampler::new(64).want(), "off: never sampled (round B)");
    fix33::set(fix33::STEP_DM, true);
    // the kernel fixture of tests32r: 2 days x 300 events, walked account-day by account-day as the day-major stage B does
    let (evs, ex, tab) = crate::tests32r::fixture_pub(2, 300, 100_000);
    let cs = crate::kern::CSet { rules: vec![(false, 0)], sl: 0 };
    let mut acc = fix33::StepAcc::default();
    for _ in 0..8 {
        // one worker thread's day: 16 x 2 x 4 = 128 account-day walks (the real stage B makes thousands per thread and day)
        let mut smp = fix33::StepSampler::new(acc.stride);
        for d in (0..16).flat_map(|_| 0..2_u16) {
            let sub: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).filter(|&e| evs[e as usize].di == d).collect();
            let subk = crate::kern::list_keys(&evs, &sub);
            for ni in 0..4 {
                let mut ac = crate::kern::Acc::new();
                let mut pc = crate::kern::Pieces::default();
                let t0 = smp.want().then(std::time::Instant::now);
                let st = crate::kern::walk_day(&evs, &ex, &tab, &sub, &subk, &cs, 0, ni, None, true, false, &mut ac, &mut pc, &[]);
                if let Some(t0) = t0 {
                    smp.push(u64::try_from(t0.elapsed().as_nanos()).unwrap(), st);
                }
            }
        }
        acc.merge(std::mem::take(&mut smp.blocks));
    }
    assert!(!acc.blocks.is_empty(), "the day-major walk is sampled: {} blocks", acc.blocks.len());
    let (p50, p99, n) = crate::kern::step_weighted(&acc.blocks);
    assert!(n == acc.blocks.len() && p50 <= p99 && p99 > 0);
    acc.dm = true;
    assert!(fix33::step_words(p50, p99, n, &acc).contains("DAY-MAJOR"));
    assert!(fix33::step_words(0, 0, 0, &acc).starts_with("NOT MEASURED"), "zero samples are never printed as 0.000");
    let mut big = fix33::StepAcc::default();
    big.merge(vec![(1, 1); fix33::STEP_CAP + 2]);
    assert!(big.blocks.len() <= fix33::STEP_CAP && big.stride == 128, "bounded: every second block kept, the stride doubled");
    reset();
}

fn sec(s: i64, h: i32, l: i32, v: u32) -> Sec {
    Sec { sod: u32::try_from(s).unwrap_or(0), high: h, low: l, vol: v }
}

/// EXTREME-MATRIX-SELLING X06 (BLOCKING): a TARGET (or time) exit decided while the price is low, whose buy-back does not fill at its first
/// limit, then a re-spike past the stop level: the rest is NOT left unwatched to 15:10; it stays on the one exit path every rule shares (the
/// K2 re-send each block, the stop's own path) and is bought back during the spike at the blocks' worst prints. Round B's engine has one exit
/// order kind for every rule (`trade::buy_path`, Kind::SignalExit), so the target's unfilled rest is exactly the stop's re-sent order.
#[test]
fn xm_unfilled_exit_rearms_stop() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let t = 36_000_i64;
    // sold at Rs 100.00 at 10:00:00; the price falls to Rs 50.00 (the target) at 10:00:20, where the exit is decided; it then jumps to Rs 140
    // (past a +5% stop at Rs 105.00) with no print inside the first limit, and keeps climbing 3% a second
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000), sec(t, 10_000, 9_950, 1_000), sec(t + 1, 10_000, 9_950, 1_000)];
    for k in 2..20 {
        rows.push(sec(t + k, 7_000, 6_000, 1_000));
    }
    rows.push(sec(t + 20, 5_000, 5_000, 1_000));
    let mut px = 14_000_i64;
    for k in 22..60 {
        rows.push(sec(t + k, i32::try_from(px).unwrap(), i32::try_from(px).unwrap(), 1_000));
        px = px * 103 / 100;
    }
    rows.push(sec(SQ_S, 30_000, 30_000, 5_000));
    rows.push(sec(SQ_S + 1, 30_000, 30_000, 5_000));
    let dl = dial().unwrap();
    for on in [false, true] {
        sw::set(sw::RESEND, on);
        let dt = DayTape::new(d, &rows, 1).unwrap();
        let tape = dt.tape().unwrap();
        let mut p = record_base(&tape, d, t, 50, 50, &dl).ok().unwrap();
        // the target exit's decision second (10:00:21: the order goes out from the block after the decision)
        let r = trip(&mut p, &tape, d, 50, 4, 50, Some(t + 21), &dl).unwrap();
        if on {
            assert_eq!((r.bb_units, r.sq_units), (50, 0), "the target's unfilled rest is re-sent every block (the stop's path) and fills during the re-spike");
            assert!(r.bb_value >= 50 * 10_500, "bought back past the +5% stop level: the spike is paid, never left open");
        } else {
            assert_eq!(r.bb_units, 0, "v2.1: one limit, passed at once: the rest rode unwatched to 15:10");
        }
    }
    let _ = ACCOUNT;
    reset();
}

/// EXTREME-MATRIX-SELLING X61 (BLOCKING; check-engine-2r: "OPEN as a named test"): the day-major STREAMED store over day files holds nothing
/// until a pass asks, loads exactly the days asked, drops them on release, and every day it streams equals the in-memory (whole) store's day;
/// the streamed passes sample the process RSS, so the run prints a peak (RUN.log "sampled RSS peak").
#[test]
fn sf31_daymajor_streamed_equals_inmemory_peak_rss_printed() {
    reset();
    let dir = std::env::current_exe().unwrap().parent().unwrap().join(format!("x61-fixture-{}", std::process::id()));
    let ymds = [20_260_302_u32, 20_260_303, 20_260_304];
    let names: Vec<String> = ymds.iter().map(|y| format!("{}-{:02}-{:02}", y / 10_000, (y / 100) % 100, y % 100)).collect();
    for (i, (&y, n)) in ymds.iter().zip(&names).enumerate() {
        let c = cube_lv(y, u64::try_from(i).unwrap() + 1);
        let mut t = crate::cube::SecTape { off: vec![0], rows: Vec::new(), lim: None };
        for ci in 0..c.cons.len() {
            for s in 0..(ci + 3) {
                t.rows.push(sec(33_300 + i64::try_from(s).unwrap(), 10_000 + i32::try_from(i).unwrap(), 9_990, u32::try_from(s + 1).unwrap()));
            }
            t.off.push(u64::try_from(t.rows.len()).unwrap());
        }
        crate::cube::write_day_t(&dir, n, &c, &t).unwrap();
    }
    let whole = crate::report3::Store::new(&dir, &names, false).unwrap();
    let mut st = crate::report3::Store::new(&dir, &names, true).unwrap();
    assert_eq!(st.loads, 0, "streamed: nothing loaded before a pass asks");
    assert!(st.cubes.iter().all(|c| c.cons.is_empty()));
    st.ensure(&[1]).unwrap();
    assert_eq!((st.loads, st.cubes[1].cons.len(), st.cubes[0].cons.len(), st.cubes[2].cons.len()), (1, 4, 0, 0), "exactly the day asked");
    for d in 0..names.len() {
        st.ensure(&[d]).unwrap();
        let (a, b) = (&st.cubes[d], &whole.cubes[d]);
        assert_eq!((a.ymd, a.cons.len(), &a.close, &a.vol, &a.cumvol, &a.cumval), (b.ymd, b.cons.len(), &b.close, &b.vol, &b.cumvol, &b.cumval), "streamed day {d} = in-memory day");
        assert_eq!((st.tapes[d].off.clone(), st.tapes[d].rows.len()), (whole.tapes[d].off.clone(), whole.tapes[d].rows.len()));
        st.release(&[]);
        assert!(st.cubes[d].cons.is_empty(), "released after its pass");
    }
    assert!(st.rss_peak_kb > 0, "the streamed passes sampled the RSS (its peak is printed)");
    reset();
}
