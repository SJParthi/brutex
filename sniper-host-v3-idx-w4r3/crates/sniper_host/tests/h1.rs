//! HOST REPAIR H1 (check-build-feat-v1-r2 section 7): the host's own decision and higher-timeframe paths on the REAL ADANIENT data
//! (Zerodha minute candles of the frozen store, the pinned common_feat repair 1's session table with its HALT rows and its CA book).
//! Red vectors, each caught by its named mutant:
//! - h1a (mutant `m_h1_reset`, the v1 host's bare `MtfSlot`): "H-RSI 15 min on 2025-11-17 09:15 is unknown"; no H-* read of any
//!   candidate after the K13 reset is a candle that ended at or before the reset; the refused reads equal common_feat's own counts
//!   on 3 minutes (15 min 4, 60 min 19; check-build-feat-v1-r2 section 4 B1).
//! - h1b (mutant `m_h1_entry`, the v1 host's bare `Session::entry`): "3 min 2020-03-13 09:21 is no candidate" (and 2020-03-23
//!   09:57); no candidate's END lies inside a market-wide halt; each refusal is counted by its reason.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use std::path::{Path, PathBuf};

use common_feat::cal::{ist_day, ist_sod, parse_ymd};
use feat_io::host::Ctx;
use feat_io::{defaults, load_ctx, minute_stream};
use sniper_host::cands::{self, Family};
use sniper_host::esn1::{Inputs, load_inputs};

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

fn ctx() -> Ctx {
    load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap()
}

fn family(c: &Ctx, inp: &Inputs, tf: &str) -> Family {
    let store = Path::new(defaults::STORE);
    let (stream, _) = minute_stream(store, "ADANIENT", tf, c);
    let (s1, _) = minute_stream(store, "ADANIENT", "1min", c);
    let facts = feat_io::host::day_facts(&s1, c);
    let lv = feat_io::host::levels(c, &facts);
    let cut = inp.run.hms("last_decision_before_ist").unwrap();
    cands::build(c, &stream, &lv, (&inp.core.0, &inp.core.1), cut).unwrap()
}

fn hm(t: i64) -> String {
    let s = ist_sod(t);
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

#[test]
fn h1a_red_vector_h_rsi_15min_on_2025_11_17_0915_is_unknown() {
    let c = ctx();
    let inp = load_inputs(&data()).unwrap();
    let mut fam = family(&c, &inp, "3min");
    let ex = parse_ymd("2025-11-17").unwrap();
    let open = c.sessions[c.index_of(ex).unwrap()].open();
    assert!(fam.resets.contains(&open), "the K13 reset of 2025-11-17 is one of the decision stream's resets: {:?}", fam.resets);
    let lad = cands::htf_ladders(&inp.dials).unwrap();
    let store = Path::new(defaults::STORE);
    let names = ["15min", "60min"];
    let mut reads = Vec::new();
    for n in names {
        let (s, _) = minute_stream(store, "ADANIENT", n, &c);
        reads.push(cands::htf_reads(&c, &s, &lad).unwrap());
    }
    cands::attach_htf(&mut fam, names.iter().map(|x| (*x).to_string()).collect(), &reads);
    let at = |i: usize, h: usize| fam.htf[i * fam.n_htf + h];
    let first = fam.cands.iter().position(|x| x.day == ex && hm(x.start) == "09:15").expect("the 2025-11-17 09:15 candidate");
    assert_eq!(at(first, 0), None, "H-* 15 min on 2025-11-17 09:15 reads the pre-ex-date candle (K13 span breach)");
    assert_eq!(at(first, 1), None, "H-* 60 min on 2025-11-17 09:15 reads the pre-ex-date candle (K13 span breach)");
    // Every candidate of the period after each reset: unknown exactly when its last completed candle ended at or before that reset.
    let mut checked = 0_u64;
    for (h, rd) in reads.iter().enumerate() {
        let (mut p, mut q) = (0_usize, 0_usize);
        for (i, x) in fam.cands.iter().enumerate() {
            // two pointers (candidates, the other timeframe's candles and the resets are each ascending)
            while rd.get(p).is_some_and(|(e, _)| *e <= x.end) {
                p += 1;
            }
            while fam.resets.get(q).is_some_and(|&r| r < x.end) {
                q += 1;
            }
            let Some(&r) = q.checked_sub(1).and_then(|k| fam.resets.get(k)) else { continue };
            let last = p.checked_sub(1).and_then(|k| rd.get(k)).map(|(e, _)| *e);
            let pre = last.is_none_or(|e| e <= r);
            assert_eq!(at(i, h).is_none(), pre, "{} {} [{}]: read {:?}, last completed end {:?}, reset {r}", x.day, hm(x.start), names[h], at(i, h), last);
            checked += 1;
        }
    }
    // every candidate after the 2025-11-17 reset (about 27,600 3-minute candidates to 2026-10) on both other timeframes
    assert!(checked > 50_000, "only {checked} cells checked");
    // common_feat's own counts on 3 minutes (check-build-feat-v1-r2 section 4, B1): 15 min 4 and 60 min 19.
    assert_eq!(fam.htf_reset_refused, vec![4, 19], "reads refused across the reset per timeframe");
}

#[test]
fn h1b_red_vector_3min_2020_03_13_0921_is_no_candidate() {
    let c = ctx();
    let inp = load_inputs(&data()).unwrap();
    let fam = family(&c, &inp, "3min");
    for (d, t) in [("2020-03-13", "09:21"), ("2020-03-23", "09:57")] {
        let day = parse_ymd(d).unwrap();
        assert!(
            !fam.cands.iter().any(|x| x.day == day && hm(x.start) == t),
            "3 min {d} {t}: its END lies inside the market-wide halt, yet it is a decision candidate"
        );
        assert!(fam.cands.iter().any(|x| x.day == day), "{d} still holds candidates outside the halt");
    }
    for x in &fam.cands {
        let s = &c.sessions[x.sess as usize];
        assert!(s.entry_at(x.end), "{} {}: a candidate the session law refuses ({:?})", x.day, hm(x.start), s.refusal(x.end));
        assert_eq!(ist_day(x.start), x.day);
    }
    assert_eq!(fam.refused.get("decision_in_halt").copied(), Some(2), "decision_in_halt counted: {:?}", fam.refused);
    assert!(fam.refused.get("session_no_entry+decision_in_halt").copied().unwrap_or(0) > 0, "2021-02-24 counted with both reasons: {:?}", fam.refused);
    assert_eq!(fam.refused.values().sum::<u64>(), fam.not_entry, "every refused candle is counted by its reason");
}
