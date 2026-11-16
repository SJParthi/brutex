//! PORTER (secfd, 2026-11-14): the seven context / calendar / range dials (X-NIFTY, X-VIX, D-EXP, D-MEND, G-52H, G-52L, G-DRNG) on
//! the SECONDS driver's compact rows, on the REAL data:
//! - `sec_eq_minutes_dials_on`: SEC-EQ on the 60-minute family with the dials ON: `ctxdials::attach` + `cands::outer_values` (the
//!   DIALS BUILD arms) against `sec::compress_family`'s context section, every value, both sides, every XE context, bit for bit.
//! - `sec_eq_seconds_dials_on`: the same on the 30-second family of 2024-03-26..2024-03-28 (a monthly expiry and a month end, the
//!   52-week window and the daily ATR known): the full family from a WHOLE-RANGE fold + `ctxdials::attach` against
//!   `sec::build_family_ctx` (`ctxdials::prepare` + the day loop's readings and range so far).
//! - `off_is_secf`: without the dials the layout, the stride and the values are secf-a's (the seven named `live_not_wired`); with
//!   them the context section is appended after every other field.
//!   Mutant `m_sec_eq` (a GE boundary byte one level short) also breaks the context section's bytes, so the two SEC-EQ tests fail
//!   under it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use std::path::{Path, PathBuf};

use common_feat::cal::parse_ymd;
use sniper_host::ctxdials::IDS;
use sniper_host::esn1::load_inputs;
use sniper_host::sec;

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

#[test]
fn sec_eq_minutes_dials_on() {
    let inp = load_inputs(&data()).unwrap();
    let out = sniper_host::secrun::sec_eq_run_ctx(&inp, "60min", None, 2, true).unwrap();
    assert!(out[0].contains("context dials ON"), "{out:?}");
    assert!(out[0].contains("mismatches 0"), "{out:?}");
    assert!(out[0].contains("PASS"), "{out:?}");
}

#[test]
fn sec_eq_seconds_dials_on() {
    let inp = load_inputs(&data()).unwrap();
    let p = Some((parse_ymd("2024-03-26").unwrap(), parse_ymd("2024-03-28").unwrap()));
    let out = sniper_host::secrun::sec_eq_run_ctx(&inp, "30s", p, 2, true).unwrap();
    assert!(out[0].contains("context dials ON"), "{out:?}");
    assert!(out[0].contains("mismatches 0"), "{out:?}");
    assert!(out[0].contains("PASS"), "{out:?}");
}

#[test]
fn off_is_secf() {
    let inp = load_inputs(&data()).unwrap();
    let off = sec::RowLayout::new(&inp.dials, 4).unwrap();
    assert!(off.ctx.is_none());
    let on = sec::RowLayout::new(&inp.dials, 4).unwrap().with_ctx(&inp.dials).unwrap();
    // X-NIFTY 2 terms, X-VIX 2, G-52H 1 x 2 variants, G-52L 1 x 2, G-DRNG 2, the flag byte: 11 bytes after secf's stride.
    assert_eq!(on.stride, off.stride + 11, "the context section is appended after every other field");
    assert_eq!(on.sess_stride, off.sess_stride);
    let fam = sec::SecFamily { tf_s: 30, lay: off, kmax: 10, ..sec::SecFamily::default() };
    for long in [true, false] {
        let (_, _, w) = sec::values(&fam, long, &inp.dials, None);
        let nw: Vec<&String> = w.not_wired.iter().filter(|x| IDS.contains(&x.as_str())).collect();
        assert_eq!(nw.len(), 7, "OFF: the seven stay live_not_wired ({long}): {:?}", w.not_wired);
        let fam_on = sec::SecFamily { tf_s: 30, lay: on.clone(), kmax: 10, ..sec::SecFamily::default() };
        let (v, specs, w2) = sec::values(&fam_on, long, &inp.dials, None);
        assert!(w2.not_wired.iter().all(|x| !IDS.contains(&x.as_str())), "ON: {:?}", w2.not_wired);
        let n_ctx = v.iter().filter(|x| IDS.contains(&inp.dials[x.dial].id.as_str())).count();
        assert_eq!(n_ctx, 110, "ON: the 110 values of the seven dials");
        assert_eq!(v.len(), specs.len());
    }
}
