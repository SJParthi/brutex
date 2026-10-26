//! IMPORT-BAD-ROW (R4-22, D-7313): every refused GDFL row is named, with its
//! file and its line, in the DURABLE AUDIT STREAM, for an options file and an
//! index file alike; and a vendor format change is named once by its shape
//! (ERR-11, D-7312).
//!
//! Its own test binary because `telemetry::install` is a process singleton:
//! this binary installs one sink, with the audit stream on (the default), and
//! reads the lines back off `audit.ndjson`, never off the rolling log. Every
//! row is invented at run time; no vendor row is quoted, and the prices are
//! far from any real one.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use brutex_core::instrument::{Exchange, InstrumentKey};
use pull::gdfl_import::{ImportKind, Run};
use pull::gdfl_nfo::NfoZips;
use pull::session::Day;

/// A scratch directory unique to `tag` and this process, empty.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-r422-audit-{tag}-{}", std::process::id()));
    let _absent = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The one sink this binary installs, at `Trace`, with the audit stream on.
fn sink() -> &'static (PathBuf, &'static telemetry::Sink) {
    static SINK: OnceLock<(PathBuf, &'static telemetry::Sink)> = OnceLock::new();
    SINK.get_or_init(|| {
        let logs = scratch("logs");
        let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
        assert!(config.audit, "the audit stream is on by default");
        let sink = telemetry::install(&config).expect("the only install in this binary");
        (logs, sink)
    })
}

/// Every DURABLE audit line holding every one of `markers`, synced first.
fn audited(markers: &[&str]) -> Vec<String> {
    let (logs, sink) = sink();
    sink.sync().unwrap();
    std::fs::read_to_string(telemetry::audit_current_path(logs))
        .unwrap_or_default()
        .lines()
        .filter(|line| markers.iter().all(|marker| line.contains(marker)))
        .map(str::to_owned)
        .collect()
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn hms(sod: u32) -> String {
    format!("{:02}:{:02}:{:02}", sod / 3_600, sod / 60 % 60, sod % 60)
}

/// One invented row: `stem,DD/MM/YYYY,HH:MM:SS,ltp,0,0,0,0,ltq,oi`.
fn row(stem: &str, d: Day, time: &str, ltp: &str, ltq: u32, oi: u32) -> String {
    format!(
        "{stem},{:02}/{:02}/{:04},{time},{ltp},0,0,0,0,{ltq},{oi}",
        d.day(),
        d.month(),
        d.year()
    )
}

/// A whole file: the modern header, `rows`, CRLF, ending in one.
fn file(rows: &[String]) -> Vec<u8> {
    let mut text = format!("{}\r\n", pull::gdfl_cm::HEADER_OPEN_INTEREST);
    for r in rows {
        text.push_str(r);
        text.push_str("\r\n");
    }
    text.into_bytes()
}

/// A STORED zip of `members` (APPNOTE method 0), central directory last.
fn stored_zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    fn le16(out: &mut Vec<u8>, v: usize) {
        out.extend_from_slice(&u16::try_from(v).unwrap().to_le_bytes());
    }
    fn le32(out: &mut Vec<u8>, v: u64) {
        out.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in members {
        let offset = u64::try_from(out.len()).unwrap();
        let crc = u64::from(pull::gdfl_archive::crc32(data));
        let len = u64::try_from(data.len()).unwrap();
        le32(&mut out, 0x0403_4b50);
        for v in [20, 0, 0] {
            le16(&mut out, v);
        }
        le32(&mut out, 0);
        le32(&mut out, crc);
        le32(&mut out, len);
        le32(&mut out, len);
        le16(&mut out, name.len());
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        le32(&mut central, 0x0201_4b50);
        for v in [20, 20, 0, 0] {
            le16(&mut central, v);
        }
        le32(&mut central, 0);
        le32(&mut central, crc);
        le32(&mut central, len);
        le32(&mut central, len);
        le16(&mut central, name.len());
        for v in [0, 0, 0, 0] {
            le16(&mut central, v);
        }
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let at = u64::try_from(out.len()).unwrap();
    let size = u64::try_from(central.len()).unwrap();
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    for v in [0, 0, members.len(), members.len()] {
        le16(&mut out, v);
    }
    le32(&mut out, size);
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// Writes `<dir>/<yyyy>.zip` holding one options day zip of `files`.
fn options_year(dir: &Path, d: Day, files: &[(String, Vec<u8>)]) {
    let folder = pull::gdfl_nfo::day_folder_name(d);
    let named: Vec<(String, &[u8])> = files
        .iter()
        .map(|(name, bytes)| (format!("{folder}\\Options\\{name}"), bytes.as_slice()))
        .collect();
    let members: Vec<(&str, &[u8])> = named.iter().map(|(n, b)| (n.as_str(), *b)).collect();
    let inner = stored_zip(&members);
    let day_zip = NfoZips::day_zip_name(d);
    let outer = stored_zip(&[(day_zip.as_str(), &inner)]);
    std::fs::write(dir.join(format!("{}.zip", d.year())), outer).unwrap();
}

/// `n` invented option rows of `stem`, one every 30 s from 09:20, the row
/// at `bad` (a 0-based row, so line `bad + 2`) written by `spoil`.
fn option_rows(
    stem: &str,
    d: Day,
    n: u32,
    bad: u32,
    spoil: &dyn Fn(String) -> String,
) -> Vec<String> {
    (0..n)
        .map(|i| {
            let r = row(stem, d, &hms(33_600 + i * 30), "4.35", 15, 9_000 + i);
            if i == bad { spoil(r) } else { r }
        })
        .collect()
}

/// **`a_bad_options_row_is_named_with_file_and_line_in_the_audit_stream`**:
/// FM-077's two measured shapes, invented: an options file whose one LTP
/// carries a letter (2018-11-06's shape) and one whose one Time is not a
/// time (2018-11-09's shape). Each file is kept with every other row (IA-4,
/// D-4816), and each refused row is named, with its FILE and its LINE, in
/// one `error` line per file in the durable audit stream: the line a reader
/// searching `/logs` for the day or the contract finds after the rolling
/// ring has moved on.
#[test]
fn a_bad_options_row_is_named_with_file_and_line_in_the_audit_stream() {
    let _installed = sink();
    for (d, ticker, bad, spoil, fault) in [
        (
            day(2018, 11, 6),
            "BANKNIFTY15NOV1877000CE",
            41_u32,
            (&|r: String| r.replacen(",4.35,", ",4O.5,", 1)) as &dyn Fn(String) -> String,
            "malformed",
        ),
        (
            day(2018, 11, 9),
            "BANKNIFTY15NOV1877100PE",
            449_u32,
            (&|r: String| r.replacen(':', ";", 1)) as &dyn Fn(String) -> String,
            "malformed",
        ),
    ] {
        let dir = scratch(&format!("nfo-{}", d.day()));
        let stem = format!("{ticker}.NFO");
        let rows = option_rows(&stem, d, 600, bad, spoil);
        options_year(&dir, d, &[(format!("{stem}.csv"), file(&rows))]);
        let store = dir.join("S");
        let run = Run {
            kind: ImportKind::Options,
            from: d,
            to: d,
            only: &[],
            store_root: &store,
        };
        let got = pull::gdfl_import::run_nfo(&NfoZips::new(&dir), &run).unwrap();
        assert!(got.failures.is_empty(), "{d}: {:?}", got.failures);
        assert_eq!(got.files, 1, "{d}: the file is kept");
        let line = bad + 2;
        let day_text = d.to_string();
        let lines = audited(&[
            "\"msg\":\"rows refused, file read\"",
            &format!("\"day\":\"{day_text}\""),
        ]);
        assert_eq!(lines.len(), 1, "{d}: one line per file: {lines:?}");
        assert!(
            lines[0].contains(&format!("{stem}.csv")),
            "{d}: the file: {}",
            lines[0]
        );
        assert!(
            lines[0].contains(&format!("\"named\":\"{line}:{fault}\"")),
            "{d}: the line: {}",
            lines[0]
        );
        assert!(lines[0].contains("\"rows_refused\":1"), "{}", lines[0]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// The index-file piece of IMPORT-BAD-ROW (DB-024's shape, invented): four
/// zero-LTP rows of a NIFTY 50 day file are refused by name, the file read,
/// and all four lines are named in ONE durable audit line.
#[test]
fn an_index_files_zero_ltp_rows_are_named_in_the_audit_stream() {
    let _installed = sink();
    let d = day(2022, 3, 7);
    let stem = "NIFTY 50.NSE_IDX";
    let mut rows: Vec<String> = (0..120)
        .map(|i| {
            row(
                stem,
                d,
                &hms(33_300 + i),
                &format!("{}.25", 77_000 + i),
                0,
                0,
            )
        })
        .collect();
    rows.insert(60, row(stem, d, &hms(33_360), "0", 0, 0));
    rows.insert(61, row(stem, d, &hms(33_360), "0.00", 0, 0));
    rows.push(row(stem, d, &hms(56_000), "77001.50", 0, 0));
    rows.push(row(stem, d, &hms(80_825), "0", 0, 0));
    rows.push(row(stem, d, &hms(80_825), "0", 0, 0));
    let folder = format!(
        "GFDLCM_INDICES_TICK_{:02}{:02}{:04}",
        d.day(),
        d.month(),
        d.year()
    );
    let name = format!("{folder}/{stem}.csv");
    let bytes = file(&rows);
    let inner = stored_zip(&[(name.as_str(), bytes.as_slice())]);
    let month = MONTHS[usize::from(d.month()) - 1];
    let outer_name = format!("INDICES/{}/{month}_{}/{folder}.zip", d.year(), d.year());
    let outer = stored_zip(&[(outer_name.as_str(), &inner)]);
    let len = u64::try_from(outer.len()).unwrap();
    let archive = pull::gdfl_archive::Archive::from_source(outer, len).unwrap();
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap();
    let read = pull::gdfl_cm::read_day(&archive, &key, d).unwrap().unwrap();
    assert_eq!(read.file.refused.total(), 4);
    let lines = audited(&[
        "\"msg\":\"rows refused, file read\"",
        "\"day\":\"2022-03-07\"",
    ]);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("NIFTY 50.NSE_IDX.csv"), "{}", lines[0]);
    assert!(
        lines[0].contains("\"named\":\"62:zero_ltp 63:zero_ltp 125:zero_ltp 126:zero_ltp\""),
        "{}",
        lines[0]
    );
}

/// ERR-11 off the durable stream: a renamed header column refuses the file
/// by its shape and the shape is named ONCE in the audit stream, with the
/// first member and the column count, however many files show it.
#[test]
fn a_renamed_header_is_named_once_by_its_shape_in_the_audit_stream() {
    let _installed = sink();
    let header = pull::gdfl_cm::HEADER_OPEN_INTEREST.replace("OpenInterest", "OpenInterestR422Z");
    let shape = pull::gdfl_cm::HeaderShape::of(header.as_bytes());
    for d in [day(2024, 4, 2), day(2024, 4, 3)] {
        let stem = "NIFTY 50.NSE_IDX";
        let mut text = format!("{header}\r\n");
        for i in 0..5 {
            text.push_str(&row(stem, d, &hms(33_300 + i), "77000.25", 0, 0));
            text.push_str("\r\n");
        }
        let folder = format!(
            "GFDLCM_INDICES_TICK_{:02}{:02}{:04}",
            d.day(),
            d.month(),
            d.year()
        );
        let name = format!("{folder}/{stem}.csv");
        let inner = stored_zip(&[(name.as_str(), text.as_bytes())]);
        let month = MONTHS[usize::from(d.month()) - 1];
        let outer_name = format!("INDICES/{}/{month}_{}/{folder}.zip", d.year(), d.year());
        let outer = stored_zip(&[(outer_name.as_str(), &inner)]);
        let len = u64::try_from(outer.len()).unwrap();
        let archive = pull::gdfl_archive::Archive::from_source(outer, len).unwrap();
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap();
        assert_eq!(
            pull::gdfl_cm::read_day(&archive, &key, d),
            Err(pull::gdfl_cm::CmRefusal::HeaderUnknown { shape })
        );
    }
    let lines = audited(&["\"msg\":\"vendor format changed\"", &shape.hex()]);
    assert_eq!(lines.len(), 1, "once per shape: {lines:?}");
    assert!(
        lines[0].contains("NIFTY 50.NSE_IDX.csv"),
        "the member: {}",
        lines[0]
    );
    assert!(
        lines[0].contains("\"day\":\"2024-04-02\""),
        "the first file: {}",
        lines[0]
    );
    assert!(lines[0].contains("\"columns\":10"), "{}", lines[0]);
    assert!(
        lines[0].contains("\"target\":\"pull.gdfl.format\"")
            || lines[0].contains("pull.gdfl.format")
    );
}
