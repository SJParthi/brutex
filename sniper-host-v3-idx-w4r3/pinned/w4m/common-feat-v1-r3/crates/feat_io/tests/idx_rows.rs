//! S4 (r3, wave 4): the instrument read rows (`feat_io::instr`): the ADANIENT row equals r2's reads byte for byte (P2's reader half:
//! OFF = xcdb), the load laws (V-RVOL refused for an index: m-IX3; X-PEER on the own underlying refused; the Muhurat rule) and the
//! Muhurat exclusion on the real 2025-10-21 session (Q39), red under `m_muhurat_kept`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use std::path::Path;

use common_feat::cal::{ist_day, parse_ymd};
use feat_io::host::{self, ZRead};
use feat_io::instr::{self, InstrRow};
use feat_io::{data_dir, defaults, load_ctx, minute_stream, minute_stream_read};

fn rows() -> std::collections::BTreeMap<String, InstrRow> {
    instr::load_default().unwrap()
}

/// The ADANIENT row loads exactly r2's context and streams (the same sessions, corporate actions, ticks, official rows, CONT
/// start; the same 3-minute and 1-minute candles and source label).
#[test]
fn rows_cash_equals_r2_load() {
    let r = &rows()["ADANIENT"];
    let a = r.load_ctx().unwrap();
    let b = load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap();
    assert_eq!(a.sessions, b.sessions);
    assert_eq!(a.ca.events(), b.ca.events());
    assert_eq!(a.ticks, b.ticks);
    assert_eq!(a.bhav, b.bhav);
    assert_eq!(a.cont_start, b.cont_start);
    assert_eq!(r.zread(), ZRead::cash("ADANIENT"));
    let store = Path::new(defaults::STORE);
    for tf in ["3min", "1min", "4min"] {
        let (x, xs) = minute_stream_read(store, &r.zread(), tf, &a);
        let (y, ys) = minute_stream(store, "ADANIENT", tf, &b);
        assert_eq!((x.tf_s, &x.source, &x.candles), (y.tf_s, &y.source, &y.candles), "{tf}");
        assert_eq!(xs, ys, "{tf}");
        assert_eq!(xs.muhurat_excluded, 0);
    }
}

/// The load laws, each refused by name: V-RVOL on an INDEX row (m-IX3), X-PEER = the row's own underlying (X-NIFTY on the NIFTY
/// track duplicates P-CHG), an INDEX row keeping the Muhurat candles; `rvol_check` refuses V-RVOL ON for the index rows only.
#[test]
fn rows_laws_refuse_by_name() {
    let text = std::fs::read_to_string(data_dir().join("FEAT-INSTR-ROWS-v1.tsv")).unwrap();
    let rr = rows();
    assert!(instr::rvol_check(&rr["NIFTY"], true).is_err(), "V-RVOL ON for NIFTY must be refused (m-IX3)");
    assert!(instr::rvol_check(&rr["BANKNIFTY"], true).is_err());
    assert!(instr::rvol_check(&rr["NIFTY"], false).is_ok());
    assert!(instr::rvol_check(&rr["ADANIENT"], true).is_ok());
    let swap = |from: &str, to: &str| text.lines().map(|l| if l.starts_with("NIFTY\t") { l.replace(from, to) } else { l.to_owned() }).collect::<Vec<_>>().join("\n");
    let e = instr::parse_rows(&swap("\texcluded\trefused\t", "\texcluded\ton\t"), &data_dir());
    assert!(e.as_ref().is_err_and(|m| m.contains("V-RVOL")), "{e:?}");
    let e2 = instr::parse_rows(&swap("\tBANKNIFTY\tofficial\t", "\tNIFTY\tofficial\t"), &data_dir());
    assert!(e2.as_ref().is_err_and(|m| m.contains("X-PEER")), "{e2:?}");
    let e3 = instr::parse_rows(&swap("\texcluded\trefused\t", "\tkept\trefused\t"), &data_dir());
    assert!(e3.as_ref().is_err_and(|m| m.contains("Muhurat")), "{e3:?}");
}

/// Q39 on the real store: Zerodha has NIFTY / BANKNIFTY 1-minute candles in the 2025-10-21 Muhurat session (13:45-14:45); the
/// index signal streams hold none of them (counted), on every intraday timeframe; the CASH row keeps ADANIENT's.
#[test]
fn muhurat_2025_10_21_excluded_from_index_streams() {
    let store = Path::new(defaults::STORE);
    let d = parse_ymd("2025-10-21").unwrap();
    for n in ["NIFTY", "BANKNIFTY"] {
        let r = &rows()[n];
        let c = r.load_ctx().unwrap();
        let i = c.index_of(d).unwrap();
        assert_eq!(c.sessions[i].kind, "MUHURAT");
        // the raw store holds the session's candles
        let raw = feat_io::zstore::read_range(store, brutex_core::vendor::Vendor::Zerodha, "INDEX", &r.zsymbol, feat_io::zstore::tf_of("1min").unwrap(), (2025, 10), (2025, 10)).0;
        let raw_n = raw.iter().filter(|b| ist_day(b.start) == d).count();
        assert!(raw_n >= 60, "{n}: {raw_n} raw 1-minute candles on 2025-10-21");
        for tf in ["1min", "3min", "5min", "60min"] {
            let (s, st) = minute_stream_read(store, &r.zread(), tf, &c);
            assert_eq!(s.candles.iter().filter(|(k, _)| *k == i).count(), 0, "{n} {tf}: a Muhurat candle reached the signal stream");
            let raw_tf = feat_io::zstore::read_range(store, brutex_core::vendor::Vendor::Zerodha, "INDEX", &r.zsymbol, feat_io::zstore::tf_of(tf).unwrap(), (2025, 10), (2025, 10)).0;
            let on_day = u64::try_from(raw_tf.iter().filter(|b| ist_day(b.start) == d).count()).unwrap();
            assert!(st.muhurat_excluded >= on_day, "{n} {tf}: counted ({} >= {on_day})", st.muhurat_excluded);
            assert!(s.source.contains("Muhurat candles EXCLUDED (Q39)"), "{}", s.source);
        }
        // the same read with the Muhurat candles kept places them (the exclusion is what removes them)
        let kept = ZRead { muhurat_excluded: false, ..r.zread() };
        let (s2, _) = host::zerodha_stream_read(store, &kept, "1min", &c, true);
        assert!(s2.candles.iter().filter(|(k, _)| *k == i).count() >= 60, "{n}: placed when kept");
    }
    let a = &rows()["ADANIENT"];
    let ca = a.load_ctx().unwrap();
    let (s, st) = minute_stream_read(store, &a.zread(), "1min", &ca);
    assert_eq!(st.muhurat_excluded, 0);
    let i = ca.index_of(d).unwrap();
    assert!(s.candles.iter().any(|(k, _)| *k == i), "ADANIENT keeps its Muhurat candles (r2 byte for byte)");
}
