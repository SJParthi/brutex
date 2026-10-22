#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
//! `cli zerodha-quarantine`: the plan read, the window, the listing check,
//! the derived buckets, plan-only writing nothing, apply writing a new
//! generation and a second apply writing nothing. R4-18. Every price is
//! invented.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::{Cause, Probe};

use super::{PLAN_HEAD, Row, ask, command_with, covering, rows_of};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-quarantine-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a root");
        Self(root)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.0);
    }
}

const SYMBOL: &str = "QFIXP";

fn path(rung: Timeframe, month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: SYMBOL,
        contract: None,
        timeframe: rung,
        month,
        file: FileKind::Bars,
    })
    .expect("a path")
}

fn dec() -> YearMonth {
    YearMonth::new(2018, 12).expect("2018-12")
}

/// The `index`-th minute from 09:15 IST on 2018-12-03.
fn minute(index: i64, level: i64) -> Bar {
    Bar {
        ts_micros: 1_543_808_700_000_000 + index * 60_000_000,
        open: level,
        high: level + 5,
        low: level - 2,
        close: level + 1,
        volume: 3 + index,
        open_interest: OI_NULL,
    }
}

fn write(root: &Path, rung: Timeframe, rows: &[Bar]) {
    let id = pull::repair_plan::symbol_id(SYMBOL);
    let mut writer = BarFile::open_or_create(root, path(rung, dec()), id).expect("opens");
    writer.append(rows).expect("commits");
}

fn served(root: &Path, rung: Timeframe) -> Vec<Bar> {
    let file = BarFile::open_existing(root, path(rung, dec()), pull::repair_plan::symbol_id(SYMBOL))
        .expect("served");
    (0..file.header().n_valid).map(|i| file.read_record(i).expect("a record")).collect()
}

fn plan_text(rows: &[(&str, &str, &str, &str, &[i64])]) -> String {
    let mut text = format!(
        "{PLAN_HEAD}\ncommit\tabc\nfeed\tzerodha\nsegment\tCASH\nscope\t{SYMBOL}\nkey\trung\tmonth\tscope\tcause\taction\tbars\ttimestamps_utc_micros\n"
    );
    for (rung, month, scope, cause, stamps) in rows {
        let listed = stamps.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
        text.push_str(&format!(
            "NSE-CASH-{SYMBOL}\t{rung}\t{month}\t{scope}\t{cause}\tquarantine\t{}\t{listed}\n",
            stamps.len()
        ));
    }
    text
}

fn write_plan(root: &Path, text: &str) -> PathBuf {
    let dir = root.join("audit").join("series-identity");
    fs::create_dir_all(&dir).expect("plan dir");
    let path = dir.join(format!("zerodha-CASH-{SYMBOL}-0011223344556677.plan"));
    fs::write(&path, text).expect("plan");
    path
}

fn run(root: &Path, arguments: &[&str]) -> (u8, String) {
    let mut out = String::new();
    let owned = root.to_path_buf();
    let code = command_with(arguments, &move || Ok(owned.clone()), &mut out);
    (code, out)
}

fn files_under(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                out.extend(files_under(&path));
            } else {
                out.push((path.clone(), fs::read(&path).expect("bytes")));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_arguments_are_read_strictly() {
    let good = ask(&["PIIND", "2018-12-01", "2020-01-31", "--cause", "foreign_level", "--reason", "two bases"])
        .expect("good");
    assert_eq!((good.cause, good.apply, good.segment), (Cause::ForeignLevel, false, "CASH"));
    assert!(ask(&["PIIND", "2018-12-01", "2020-01-31", "--cause", "foreign_level", "--reason", "r", "--apply"])
        .expect("apply")
        .apply);
    for bad in [
        &["--plan", "2018-12-01", "2020-01-31"][..],
        &["PIIND", "2018-12-01"][..],
        &["PIIND", "2020-01-31", "2018-12-01", "--cause", "foreign_level", "--reason", "r"][..],
        &["PIIND", "2018-13-01", "2020-01-31", "--cause", "foreign_level", "--reason", "r"][..],
        &["PIIND", "2018-12-01", "2020-01-31", "--cause", "foreign", "--reason", "r"][..],
        &["PIIND", "2018-12-01", "2020-01-31", "--reason", "r"][..],
        &["PIIND", "2018-12-01", "2020-01-31", "--cause", "zero_price"][..],
        &["PIIND", "2018-12-01", "2020-01-31", "--cause", "zero_price", "--reason", ""][..],
        &["PIIND", "2018-12-01", "2020-01-31", "--cause", "zero_price", "--reason", "r", "--plan", "--apply"][..],
        &["PIIND", "2018-12-01", "2020-01-31", "--cause", "zero_price", "--reason", "r", "--segment", "NFO"][..],
        &["PIIND", "2018-12-01", "2020-01-31", "--cause", "zero_price", "--reason", "r", "--bogus"][..],
    ] {
        assert!(ask(bad).is_err(), "{bad:?} must be refused");
    }
    let root = Root::new("misuse");
    assert_eq!(run(&root.0, &["PIIND"]).0, MISUSED);
}

#[test]
fn the_plan_rows_are_taken_by_symbol_cause_action_and_window() {
    let asked = ask(&[SYMBOL, "2018-12-01", "2019-01-31", "--cause", "foreign_level", "--reason", "r"]).expect("ask");
    let text = plan_text(&[
        ("1min", "2018-12", "timestamps", "foreign_level", &[1, 2]),
        ("1min", "2018-11", "timestamps", "foreign_level", &[3]),
        ("1min", "2019-01", "whole_month", "foreign_level", &[]),
        ("1min", "2018-12", "timestamps", "zero_price", &[4]),
        ("1day", "2018-12", "timestamps", "foreign_level", &[5]),
    ]);
    let rows = rows_of(&text, &asked).expect("rows");
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0], Row { symbol: SYMBOL.to_owned(), rung: Timeframe::MINUTE_1, month: dec(), stamps: Some(vec![1, 2]) });
    assert_eq!(rows[1].stamps, None, "a whole month names no stamp");
    assert_eq!(rows[2].rung, Timeframe::DAY_1);
    assert!(rows_of("not a plan\n", &asked).is_err(), "a foreign head is refused");
    let miscounted = plan_text(&[("1min", "2018-12", "timestamps", "foreign_level", &[1, 2])]).replace("quarantine\t2", "quarantine\t3");
    assert!(rows_of(&miscounted, &asked).is_err(), "a row counting other than it lists is refused");
}

#[test]
fn a_derived_bucket_is_named_when_its_span_holds_a_named_minute() {
    let five: Vec<Bar> = (0..4).map(|i| minute(i * 5, 100)).collect();
    assert_eq!(covering(&five, 300, &[]), Vec::<i64>::new());
    assert_eq!(covering(&five, 300, &[minute(6, 0).ts_micros]), vec![five[1].ts_micros]);
    assert_eq!(
        covering(&five, 300, &[minute(0, 0).ts_micros, minute(4, 0).ts_micros, minute(19, 0).ts_micros]),
        vec![five[0].ts_micros, five[3].ts_micros]
    );
    assert_eq!(covering(&five, 300, &[minute(25, 0).ts_micros]), Vec::<i64>::new(), "past the last bucket");
}

#[test]
fn plan_writes_nothing_and_apply_drops_the_named_minutes_and_their_buckets_once() {
    let root = Root::new("apply");
    let minutes: Vec<Bar> = (0..10).map(|i| minute(i, if i == 2 || i == 3 { 8_910 } else { 410 })).collect();
    write(&root.0, Timeframe::MINUTE_1, &minutes);
    let buckets: Vec<Bar> = (0..2).map(|i| minute(i * 5, 410)).collect();
    write(&root.0, Timeframe::from_secs(300).expect("5min"), &buckets);
    let named = [minutes[2].ts_micros, minutes[3].ts_micros];
    write_plan(&root.0, &plan_text(&[("1min", "2018-12", "timestamps", "foreign_level", &named)]));
    let base = [SYMBOL, "2018-12-01", "2018-12-31", "--cause", "foreign_level", "--reason", "invented two bases"];
    let before = files_under(&root.0);
    let (code, out) = run(&root.0, &base);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("plan: 2 bar(s) would be set aside"), "{out}");
    assert_eq!(files_under(&root.0), before, "--plan wrote nothing");
    let applied = [&base[..], &["--apply"]].concat();
    let (code, out) = run(&root.0, &applied);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("DROPPED 2 of 10"), "{out}");
    assert!(out.contains("5min 2018-12: 1 bucket(s)"), "{out}");
    let kept: Vec<Bar> = minutes.iter().filter(|b| b.open != 8_910).copied().collect();
    assert_eq!(served(&root.0, Timeframe::MINUTE_1), kept);
    assert_eq!(served(&root.0, Timeframe::from_secs(300).expect("5min")), vec![buckets[1]]);
    assert!(matches!(
        store::quarantine::probe(&root.0, path(Timeframe::MINUTE_1, dec())),
        Ok(Probe::Quarantined(entry)) if entry.bars == 2
    ));
    let after = files_under(&root.0);
    let (code, out) = run(&root.0, &applied);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("nothing written (already_dropped)"), "{out}");
    assert_eq!(files_under(&root.0), after, "a second apply wrote nothing");
}

#[test]
fn a_before_listing_plan_with_no_master_on_disk_is_refused_by_name() {
    let root = Root::new("nomaster");
    let minutes: Vec<Bar> = (0..3).map(|i| minute(i, 99_000)).collect();
    write(&root.0, Timeframe::MINUTE_1, &minutes);
    write_plan(&root.0, &plan_text(&[("1min", "2018-12", "whole_month", "before_listing", &[])]));
    let before = files_under(&root.0.join("bars"));
    let (code, out) = run(
        &root.0,
        &[SYMBOL, "2018-12-01", "2018-12-31", "--cause", "before_listing", "--reason", "pre-listing", "--apply"],
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("REFUSED") && out.contains("listing day"), "{out}");
    assert_eq!(files_under(&root.0.join("bars")), before);
}

#[test]
fn a_whole_zero_price_month_is_recorded_not_published() {
    let root = Root::new("zero");
    let zero = Bar { ts_micros: minute(0, 0).ts_micros - 33_300_000_000, open: 0, high: 0, low: 0, close: 0, volume: 7, open_interest: OI_NULL };
    let day = YearMonth::new(2018, 12).expect("month");
    let id = pull::repair_plan::symbol_id(SYMBOL);
    let day_path = path(Timeframe::DAY_1, day);
    match BarFile::open_or_create(&root.0, day_path, id).and_then(|mut writer| writer.append(&[zero])) {
        // A WRITE DOOR THAT REFUSES A ZERO BAR (PREC-16) leaves nothing to
        // quarantine: the run then answers that the plan names a month the
        // store does not hold, by name.
        Err(_) => {
            write_plan(&root.0, &plan_text(&[("1day", "2018-12", "whole_month", "zero_price", &[])]));
            let (code, out) = run(&root.0, &[SYMBOL, "2018-12-01", "2018-12-31", "--cause", "zero_price", "--reason", "zero row", "--apply"]);
            assert_eq!(code, FAILED, "{out}");
        }
        Ok(()) => {
            write_plan(&root.0, &plan_text(&[("1day", "2018-12", "whole_month", "zero_price", &[])]));
            let (code, out) = run(&root.0, &[SYMBOL, "2018-12-01", "2018-12-31", "--cause", "zero_price", "--reason", "zero row", "--apply"]);
            assert_eq!(code, OK, "{out}");
            assert!(out.contains("RECORDED WHOLE: 1 bar(s)"), "{out}");
            assert!(!root.0.join("bar-revisions-v1").exists());
        }
    }
}

#[test]
fn no_plan_on_disk_is_refused_by_name() {
    let root = Root::new("noplan");
    let (code, out) = run(&root.0, &[SYMBOL, "2018-12-01", "2018-12-31", "--cause", "zero_price", "--reason", "r"]);
    assert_eq!(code, FAILED);
    assert!(out.contains("no quarantine plan"), "{out}");
}
