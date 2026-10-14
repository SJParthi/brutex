//! `cli zerodha-gaps` — the gap ledger for Zerodha months already stored.
//! R3-18; findings DB-A12, DB-G46, CMP-19.
//!
//! # Usage
//!
//! ```text
//! cli zerodha-gaps INSTRUMENT|ALL FROM TO [--segment INDEX|CASH] [--apply]
//! ```
//!
//! `FROM` and `TO` are `YYYY-MM-DD`, inclusive; every held `1min` month that
//! touches them is read. `--segment` is `INDEX` unless named.
//!
//! # What it does
//!
//! For each series (one, or every series of the segment in name order) it
//! reads the stored minute months and the calendar of record — **never the
//! vendor** — and finds each session the store holds short, by the repair
//! plan's rule (`pull::ingest::repair_plan`). It prints one row per series:
//! the months read, the sessions of record, the short ones, the lines the
//! ledger lacks, their missing minutes, and the sessions it will not write a
//! line for and why (`already` named by the ledger, `before_first`, the
//! newest month's tail `frontier`, length `unjudged`, `unattested`). For one
//! instrument it prints every line besides.
//!
//! **Without `--apply` it writes nothing.** With `--apply` it appends each
//! series' lines to `<store>/audit/zerodha.gaps` in one write and one sync
//! (`pull::ingest::gap_backfill`), so `/db`'s day band reads a reason
//! (`why=stored_short`, with `fill=`) for a stored hole instead of "no
//! ledger". A second `--apply` finds every line the first wrote and writes
//! nothing. No bar, census row or sidecar is opened for writing.
//!
//! # Exit
//!
//! [`OK`] when every month was read (and, applying, every line written);
//! [`FAILED`] when a month would not open or the ledger would not take a
//! line, each named; [`MISUSED`] for arguments that do not name a series and
//! a window, before anything is opened.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::ingest::gap_backfill::{self, Have};
use pull::session::{Day, Window};
use store::path::{FileKind, PathParts, Timeframe};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli zerodha-gaps INSTRUMENT|ALL FROM TO [--segment INDEX|CASH] \
     [--apply]  (FROM and TO are YYYY-MM-DD, inclusive)";

/// The most lines printed for one instrument; the rest are counted.
const MAX_PRINTED: usize = 200;

/// What the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    /// One store symbol, or `None` for every series of the segment.
    instrument: Option<String>,
    segment: &'static str,
    window: Window,
    apply: bool,
}

fn day(label: &str, word: &str) -> Result<Day, String> {
    let digits = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u16>().ok())
            .flatten()
    };
    let mut parts = word.split('-');
    let parsed = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d), None) => digits(y, 4).zip(digits(m, 2)).zip(digits(d, 2)),
        _ => None,
    };
    let Some(((year, month), date)) = parsed else {
        return Err(format!(
            "{label} `{}` is not a YYYY-MM-DD day",
            crate::stored::clipped(word)
        ));
    };
    Day::new(
        year,
        u8::try_from(month).unwrap_or(u8::MAX),
        u8::try_from(date).unwrap_or(u8::MAX),
    )
    .map_err(|why| format!("{label} `{}`: {why}", crate::stored::clipped(word)))
}

/// The arguments, refused by name before anything is opened.
fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let [instrument, from, to, flags @ ..] = arguments else {
        return Err(format!(
            "zerodha-gaps takes INSTRUMENT FROM TO and flags; it was given {} argument(s)",
            arguments.len()
        ));
    };
    let mut segment = "INDEX";
    let mut apply = false;
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        match *flag {
            "--apply" if !apply => apply = true,
            "--segment" => {
                let value = rest.next().ok_or("--segment takes a value")?;
                segment = match value.to_ascii_uppercase().as_str() {
                    "INDEX" => "INDEX",
                    "CASH" => "CASH",
                    _ => {
                        return Err(format!(
                            "--segment `{}`: INDEX or CASH",
                            crate::stored::clipped(value)
                        ));
                    }
                };
            }
            other => {
                return Err(format!(
                    "`{}` is not a flag of zerodha-gaps, or is given twice",
                    crate::stored::clipped(other)
                ));
            }
        }
    }
    let window = Window::new(day("FROM", from)?, day("TO", to)?).map_err(|why| why.to_string())?;
    let instrument = if instrument.eq_ignore_ascii_case("ALL") {
        None
    } else {
        let symbol = instrument.to_ascii_uppercase();
        brutex_core::symbol::Symbol::new(&symbol)
            .map_err(|why| format!("`{}`: {why}", crate::stored::clipped(instrument)))?;
        Some(symbol)
    };
    Ok(Ask {
        instrument,
        segment,
        window,
        apply,
    })
}

/// Runs the verb against the store root `store` names, so every arm is driven
/// by a test without the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let ask = match ask(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let root = match store() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(
                out,
                "ZERODHA GAPS REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    run(&root, &ask, out)
}

/// `cli zerodha-gaps`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // AN `--apply` BY AN UNSTAMPED BUILD IS REFUSED (DB-B71); a read is not.
    let store = crate::store_write::root_for("zerodha-gaps", arguments);
    command_with(arguments, &store, out)
}

/// The symbols to read: the one named, or every series of the segment in
/// name order.
fn symbols(root: &Path, ask: &Ask) -> Result<Vec<String>, String> {
    if let Some(symbol) = &ask.instrument {
        return Ok(vec![symbol.clone()]);
    }
    let dir = root
        .join("bars")
        .join(Vendor::Zerodha.as_str())
        .join("NSE")
        .join(ask.segment);
    let entries =
        std::fs::read_dir(&dir).map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| brutex_core::symbol::Symbol::new(name).is_ok())
        .collect();
    names.sort_unstable();
    Ok(names)
}

/// One table row: a series' counts, or the total's.
fn row(out: &mut String, name: &str, t: &gap_backfill::Tally, lines: usize) {
    let _ = writeln!(
        out,
        "  {name:<20}  {:>6}  {:>8}  {:>5}  {lines:>5}  {:>7}  {:>7}  {:>12}  {:>8}  {:>8}  {:>10}",
        t.months,
        t.sessions,
        t.short,
        t.minutes,
        t.already,
        t.before_first,
        t.frontier,
        t.length_unjudged,
        t.unattested
    );
}

/// `t` added into `total`; refusals are printed by their series, not summed.
fn add(total: &mut gap_backfill::Tally, t: &gap_backfill::Tally) {
    total.months += t.months;
    total.sessions += t.sessions;
    total.short += t.short;
    total.minutes += t.minutes;
    total.already += t.already;
    total.before_first += t.before_first;
    total.frontier += t.frontier;
    total.length_unjudged += t.length_unjudged;
    total.unattested += t.unattested;
}

/// The run's header: what is read, from where, and what the ledger holds.
fn header(out: &mut String, root: &Path, ask: &Ask, series: usize, have: &Have) {
    let _ = writeln!(
        out,
        "ZERODHA STORED-MONTH GAP LEDGER ({}) — NSE/{} 1min, {}..{}: {series} series under {}; no vendor call",
        if ask.apply {
            "--apply: lines are appended"
        } else {
            "read-only"
        },
        ask.segment,
        ask.window.from(),
        ask.window.to(),
        root.display()
    );
    let _ = writeln!(
        out,
        "  ledger {}: {} day(s) already named{}",
        gap_backfill::ledger_path(root, Vendor::Zerodha).display(),
        have.len(),
        if have.faults.is_empty() {
            String::new()
        } else {
            format!(
                "; {} line(s) not read as a gap line (first at line {})",
                have.faults.len(),
                have.faults.first().copied().unwrap_or_default()
            )
        }
    );
    let _ = writeln!(
        out,
        "  instrument            months  sessions  short  lines  minutes  already  before_first  frontier  unjudged  unattested"
    );
}

fn run(root: &Path, ask: &Ask, out: &mut String) -> u8 {
    let vendor = Vendor::Zerodha;
    let mut have: Have = match gap_backfill::ledgered(root, vendor) {
        Ok(have) => have,
        Err(why) => {
            let _ = writeln!(
                out,
                "ZERODHA GAPS REFUSED: the ledger cannot be read: {why}. Nothing was written."
            );
            return FAILED;
        }
    };
    let symbols = match symbols(root, ask) {
        Ok(symbols) => symbols,
        Err(why) => {
            let _ = writeln!(out, "ZERODHA GAPS REFUSED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    header(out, root, ask, symbols.len(), &have);
    let mut failed = false;
    let mut total = gap_backfill::Tally::default();
    let mut total_lines = 0_usize;
    for symbol in &symbols {
        let Ok(month) = ask.window.from().year_month() else {
            continue;
        };
        let parts = PathParts {
            vendor,
            exchange: "NSE",
            segment: ask.segment,
            symbol,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        };
        let series = gap_backfill::series(root, parts, ask.window, &have);
        if series.tally.months == 0 && ask.instrument.is_none() {
            continue;
        }
        row(out, symbol, &series.tally, series.lines.len());
        for why in &series.tally.refused {
            failed = true;
            let _ = writeln!(out, "    REFUSED {why}");
        }
        if ask.instrument.is_some() {
            for line in series.lines.iter().take(MAX_PRINTED) {
                let _ = writeln!(out, "    {line}");
            }
            if series.lines.len() > MAX_PRINTED {
                let _ = writeln!(out, "    ... and {} more", series.lines.len() - MAX_PRINTED);
            }
        }
        if ask.apply
            && !series.lines.is_empty()
            && let Err(why) = gap_backfill::write(root, vendor, &series, &mut have)
        {
            failed = true;
            let _ = writeln!(out, "    NOT WRITTEN: {why}");
        }
        total_lines += series.lines.len();
        add(&mut total, &series.tally);
    }
    row(out, "TOTAL", &total, total_lines);
    let _ = writeln!(
        out,
        "{}",
        match (ask.apply, failed) {
            (true, false) => format!(
                "{total_lines} line(s) appended (why=stored_short); a second --apply writes nothing."
            ),
            (true, true) =>
                "SOME LINES OR MONTHS FAILED, each named above; every other line was appended."
                    .to_owned(),
            (false, _) =>
                format!("Nothing was written. --apply appends these {total_lines} line(s)."),
        }
    );
    if failed { FAILED } else { OK }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test-only assertions"
)]
mod tests {
    use super::*;
    use pull::ingest::repair_plan::{self, Owes};
    use pull::session::IST_OFFSET_SECS;
    use store::format::{Bar, OI_NULL};
    use store::path::{StorePath, YearMonth};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("brutex-ZGAPS-{tag}-{}", std::process::id()));
            let _fresh = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _cleanup = std::fs::remove_dir_all(&self.0);
        }
    }

    /// `symbol`'s 2019-09 minute month, every owed minute held but 15:15-15:28
    /// on the 23rd. Invented prices.
    fn month(root: &Path, symbol: &str) {
        let parts = PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2019, 9).unwrap(),
            file: FileKind::Bars,
        };
        let mut bars = Vec::new();
        let (from, to) = (
            Day::new(2019, 9, 1).unwrap(),
            Day::new(2019, 9, 30).unwrap(),
        );
        for n in from.days_from_epoch()..=to.days_from_epoch() {
            let Owes::Windows((windows, count)) = repair_plan::owed("INDEX", n) else {
                continue;
            };
            for &(open, close) in &windows[..count] {
                for m in open..close {
                    if n == Day::new(2019, 9, 23).unwrap().days_from_epoch()
                        && (915..929).contains(&m)
                    {
                        continue;
                    }
                    let secs = i64::from(n) * 86_400 - IST_OFFSET_SECS + i64::from(m) * 60;
                    bars.push(Bar {
                        ts_micros: secs * 1_000_000,
                        open: 123_400,
                        high: 123_450,
                        low: 123_350,
                        close: 123_410,
                        volume: 0,
                        open_interest: OI_NULL,
                    });
                }
            }
        }
        let id = repair_plan::symbol_id(symbol);
        let mut file =
            store::file::BarFile::open_or_create(root, StorePath::new(parts).unwrap(), id).unwrap();
        file.append(&bars).unwrap();
    }

    fn go(root: &Path, arguments: &[&str]) -> (u8, String) {
        let mut out = String::new();
        let at = root.to_path_buf();
        let code = command_with(arguments, &move || Ok(at.clone()), &mut out);
        (code, out)
    }

    fn ledger(root: &Path) -> Option<String> {
        std::fs::read_to_string(root.join("audit").join("zerodha.gaps")).ok()
    }

    /// Misuse is refused by name before the store is opened.
    #[test]
    fn misuse_is_refused_before_anything_is_opened() {
        let never = || -> Result<PathBuf, String> { panic!("the store must not be opened") };
        for bad in [
            &[][..],
            &["NIFTY", "2019-09-01"][..],
            &["NIFTY", "2019-9-01", "2019-09-30"][..],
            &["NIFTY", "2019-09-30", "2019-09-01"][..],
            &["NIFTY", "2019-09-01", "2019-09-30", "--segment", "FUT"][..],
            &["NIFTY", "2019-09-01", "2019-09-30", "--segment"][..],
            &["NIFTY", "2019-09-01", "2019-09-30", "--apply", "--apply"][..],
            &["NIFTY", "2019-09-01", "2019-09-30", "--from-store", "x"][..],
            &["nifty!", "2019-09-01", "2019-09-30"][..],
        ] {
            let mut out = String::new();
            assert_eq!(command_with(bad, &never, &mut out), MISUSED, "{bad:?}");
            assert!(out.starts_with("refused: ") && out.contains(USAGE), "{out}");
        }
        let mut out = String::new();
        let gone = || -> Result<PathBuf, String> { Err("no store here".to_owned()) };
        assert_eq!(
            command_with(&["ALL", "2019-09-01", "2019-09-30"], &gone, &mut out),
            FAILED
        );
        assert!(out.contains("no store here") && out.contains("Nothing was read or written"));
    }

    /// Read-only by default; `--apply` writes the lines once; a second
    /// `--apply` writes nothing; `ALL` reads every series in name order.
    #[test]
    fn the_verb_reads_then_writes_once_and_is_idempotent() {
        let scratch = Scratch::new("RUN");
        month(&scratch.0, "NIFTY");
        month(&scratch.0, "BANKNIFTY");
        let (code, out) = go(&scratch.0, &["nifty", "2019-09-01", "2019-09-30"]);
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains("read-only")
                && out.contains("Nothing was written. --apply appends these 1 line(s)."),
            "{out}"
        );
        assert!(out.contains(
            "gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=stored_short"
        ), "{out}");
        assert_eq!(ledger(&scratch.0), None, "a read writes nothing");

        let (code, out) = go(&scratch.0, &["ALL", "2019-09-01", "2019-09-30", "--apply"]);
        assert_eq!(code, OK, "{out}");
        let bank = out.find("  BANKNIFTY ").expect("BANKNIFTY's row");
        let nifty = out.find("  NIFTY ").expect("NIFTY's row");
        assert!(bank < nifty, "name order");
        assert!(out.contains("2 line(s) appended"), "{out}");
        let written = ledger(&scratch.0).unwrap();
        assert_eq!(written.lines().count(), 2);
        assert!(written.lines().all(
            |l| l.contains(" why=stored_short ") && l.ends_with(" fill=interior source=store")
        ));

        let (code, out) = go(&scratch.0, &["ALL", "2019-09-01", "2019-09-30", "--apply"]);
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains("0 line(s) appended") && out.contains("2 day(s) already named"),
            "{out}"
        );
        assert_eq!(ledger(&scratch.0).unwrap(), written);
    }

    /// A month that will not open is named and fails the run; the other
    /// series' lines are still found.
    #[test]
    fn a_month_that_will_not_open_is_named() {
        let scratch = Scratch::new("TORN");
        month(&scratch.0, "NIFTY");
        let dir = scratch.0.join("bars/zerodha/NSE/INDEX/BROKEN/1min");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("2019-09.bin"), b"torn").unwrap();
        let (code, out) = go(&scratch.0, &["ALL", "2019-09-01", "2019-09-30"]);
        assert_eq!(code, FAILED, "{out}");
        assert!(out.contains("    REFUSED 2019-09: "), "{out}");
        assert!(out.contains("  NIFTY "), "{out}");
        let (code, out) = go(
            &scratch.0,
            &["ALL", "2019-09-01", "2019-09-30", "--segment", "CASH"],
        );
        assert_eq!(code, FAILED);
        assert!(out.contains("cannot list"), "{out}");
    }
}
