//! `cli zerodha-verify` — the cold Zerodha verification (V-3): every stored
//! rung of every day of a span checked from the store alone, spending no
//! quota, its records spooled and then merged (IA-9; D-4811, D-4857, D-4858).
//!
//! # Usage
//!
//! ```text
//! cli zerodha-verify FROM TO [INSTRUMENT ...] [--no-merge] [--workers N]
//! ```
//!
//! `FROM` and `TO` are `YYYY-MM-DD`, inclusive. Each `INSTRUMENT` is a store
//! key (`NSE-INDEX-NIFTY`) or a symbol (`NIFTY`); none checks every Zerodha
//! series held. `--workers N` (1 to 8) checks N series side by side; without
//! it the host's parallelism, at most 8 (RA-145), is used, and the answer is
//! the same whatever the count.
//!
//! # What it does
//!
//! Takes `<store>/verify/zerodha.lock` (a second cold run exits `run_busy`),
//! a run id from the runs ledger (mode `zerodha-verify`), reads the gap
//! ledger once, and for every held series and every calendar-open day of the
//! span, whether or not a window landed that day, verifies 1min (against a
//! kept answer where one exists, else `no_response_kept`), every derived
//! rung refolded under D-3321, and 1day with `daycheck`, classifying each
//! missing minute by the ledger's reason (`pull::zerodha_verify::cold`). The
//! records go to the run's spool, and the spool is merged in this process
//! under `verify/merge.lock` (`--no-merge` leaves it for `cli verify-merge`).
//! It prints one table of records per rung and verdict code, and the first
//! records that are not `match`, in words.
//!
//! It writes only `verify/` (the spool, its merge log and the verdict
//! images) and one runs-ledger record: never a bar, a census row or a
//! sidecar.
//!
//! # Exit
//!
//! [`OK`] when the span was verified and merged; [`FAILED`] for `run_busy`,
//! a month that would not open, a month a pull held past
//! `MONTH_BUSY_RETRY` (its records `not_compared (month_busy)`, printed
//! `BUSY`), a spool that would not take a record, or a merge that did not
//! finish; [`MISUSED`] for arguments that do not name a span.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pull::gdfl_run::stop::StopControl;
use pull::session::Day;
use pull::zerodha_verify::{self, ColdRequest, MergeError, spool::SpoolWriter};
use store::verdict::Mode;

use crate::verify_merge_command::{render, run_id};
use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli zerodha-verify FROM TO [INSTRUMENT ...] [--no-merge] [--workers N]  (FROM and TO are YYYY-MM-DD; N is 1 to 8)";

/// The cold run's own lock (design §5.3).
pub(crate) const LOCK: &str = "verify/zerodha.lock";

/// The stop file a cold run reads before each month.
pub(crate) const STOP_FILE: &str = "verify/zerodha.stop";

/// What the arguments ask.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    request: ColdRequest,
    merge: bool,
    /// The workers the series are checked on (RA-145): `--workers N`, else
    /// the host's parallelism, at most `MAX_COLD_WORKERS`.
    workers: usize,
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
    let [from, to, rest @ ..] = arguments else {
        return Err(format!(
            "zerodha-verify takes FROM TO and instruments; it was given {} argument(s)",
            arguments.len()
        ));
    };
    let (from, to) = (day("FROM", from)?, day("TO", to)?);
    if to < from {
        return Err(format!("TO {to} is before FROM {from}"));
    }
    let mut merge = true;
    let mut workers = None;
    let mut only = Vec::with_capacity(rest.len());
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        match *word {
            "--no-merge" if merge => merge = false,
            "--workers" if workers.is_none() => {
                let count = words
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
                    .filter(|n| (1..=zerodha_verify::MAX_COLD_WORKERS).contains(n))
                    .ok_or_else(|| {
                        format!(
                            "--workers takes a count from 1 to {}",
                            zerodha_verify::MAX_COLD_WORKERS
                        )
                    })?;
                workers = Some(count);
            }
            flag if flag.starts_with('-') => {
                return Err(format!(
                    "`{}` is not a flag of zerodha-verify, or is given twice",
                    crate::stored::clipped(flag)
                ));
            }
            name if name.is_empty() || name.len() > 64 || name.contains('/') => {
                return Err(format!(
                    "`{}` is not a store key or a symbol",
                    crate::stored::clipped(name)
                ));
            }
            name => only.push(name.to_owned()),
        }
    }
    Ok(Ask {
        request: ColdRequest { from, to, only },
        merge,
        workers: workers.unwrap_or_else(zerodha_verify::cold_workers),
    })
}

/// [`command`] over a named store root, so every arm is driven by a test.
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
                "ZERODHA-VERIFY REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    run(&root, &ask, out)
}

/// `cli zerodha-verify`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let store = crate::store_write::root_for("zerodha-verify", arguments);
    command_with(arguments, &store, out)
}

fn run(root: &Path, ask: &Ask, out: &mut String) -> u8 {
    let file = match pull::gdfl_run::locks::open_lock_file(root, LOCK) {
        Ok(file) => file,
        Err(why) => {
            let _ = writeln!(out, "ZERODHA-VERIFY REFUSED: {why}");
            return FAILED;
        }
    };
    // BUSY IS NOT BROKEN: another holder is `run_busy`; a lock the host
    // refused is named as that refusal, never read as someone else's run.
    let lock = match store::flock::Flock::try_lock(file, root.join(LOCK)) {
        Ok(lock) => lock,
        Err(std::fs::TryLockError::WouldBlock) => {
            let _ = writeln!(
                out,
                "ZERODHA-VERIFY run_busy: another cold verification holds {LOCK} ({})",
                store::flock::holder_note(&root.join(LOCK))
            );
            return FAILED;
        }
        Err(std::fs::TryLockError::Error(why)) => {
            let _ = writeln!(out, "ZERODHA-VERIFY REFUSED: {LOCK} would not lock: {why}");
            return FAILED;
        }
    };
    let run = match run_id(root, "zerodha-verify") {
        Ok(run) => run,
        Err(why) => {
            let _ = writeln!(
                out,
                "ZERODHA-VERIFY REFUSED: the runs ledger gave no run id: {why}. Nothing was verified."
            );
            return FAILED;
        }
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let writer = SpoolWriter::new(root, run, Mode::ZerodhaCold, now);
    let stop = StopControl::new(root.join(STOP_FILE), run);
    let progress = pull::gdfl_run::ProgressFile::of_record(
        root.join(zerodha_verify::PROGRESS_COLD),
        zerodha_verify::progress_record(
            "zerodha-verify",
            1,
            run,
            ask.workers,
            Some((ask.request.from, ask.request.to)),
        ),
    );
    let started = std::time::Instant::now();
    let report = match zerodha_verify::cold_observed(
        root,
        &ask.request,
        &writer,
        Some(&stop),
        ask.workers,
        Some(&progress),
    ) {
        Ok(report) => report,
        Err(why) => {
            // THE END IS WRITTEN WHILE THE LOCK IS HELD, so a run that
            // follows never has its record written over by this one.
            zerodha_verify::progress_end(&progress, false, Some(&why));
            crate::verify_merge_command::progress_said(&progress, out);
            let _ = writeln!(
                out,
                "ZERODHA-VERIFY run {run} REFUSED: {why}. Records spooled before it wait in verify/spool/{run}."
            );
            return FAILED;
        }
    };
    zerodha_verify::progress_end(&progress, report.stopped, None);
    crate::verify_merge_command::progress_said(&progress, out);
    // THE RUN ENDS HERE: its writer, and with it the run's live lock, goes
    // before any merge, or every merger would leave this run's spools
    // waiting as a run still spooling (IA-9 repair 1).
    drop(writer);
    drop(lock);
    let took = started.elapsed().as_millis();
    let mut code = render_report(run, ask, &report, took, out);
    if !ask.merge {
        let _ = writeln!(
            out,
            "  --no-merge: the spool waits in verify/spool/{run}; `cli verify-merge` merges it"
        );
        return code;
    }
    if !merge_after(root, run, out) {
        code = FAILED;
    }
    code
}

/// The cold run's table: records per rung and code, the first that are not
/// `match`, and every refusal; [`OK`] unless a month was refused or a stop
/// asked.
fn render_report(
    run: u64,
    ask: &Ask,
    report: &zerodha_verify::ColdReport,
    took: u128,
    out: &mut String,
) -> u8 {
    let _ = writeln!(
        out,
        "ZERODHA-VERIFY run {run}: {} to {}, {} series, {} month(s), {} day(s), {} record(s) in {took} ms on {} worker(s)",
        ask.request.from,
        ask.request.to,
        report.series,
        report.months,
        report.days,
        report.records,
        report.workers,
    );
    let _ = writeln!(out, "  {:<8} {:<26} {:>9}", "rung", "verdict", "records");
    for (rung, code, count) in &report.by_rung {
        let _ = writeln!(out, "  {rung:<8} {:<26} {count:>9}", code.word());
    }
    // EVERY UNIT THE STORE LACKS, BY ITS CLASS: minutes at 1min, buckets at
    // a derived rung, the bar at 1day (DB-A12: a vendor's hole, the
    // ledger's reason, or neither vendor's).
    if !report.missing.is_empty() {
        let _ = writeln!(
            out,
            "  {:<8} {:<48} {:>9}",
            "rung", "verdict (reason) of the units lacking", "units"
        );
        for (rung, code, reason, units) in &report.missing {
            let said = pull::verdict_words::reason_word(*reason).map_or_else(
                || code.word().to_owned(),
                |w| format!("{} ({w})", code.word()),
            );
            let _ = writeln!(out, "  {rung:<8} {said:<48} {units:>9}");
        }
    }
    for (_, _, example) in &report.examples {
        let _ = writeln!(out, "  {example}");
    }
    for refused in &report.refused {
        let _ = writeln!(out, "  REFUSED {refused}");
    }
    // A MONTH A WRITER HELD IS NOT A STORE FAULT, AND NOT A SPAN COMPARED:
    // its records read `not_compared (month_busy)`; said apart, and the run
    // is not OK, so a span with such a month is run again.
    for busy in &report.busy {
        let _ = writeln!(out, "  BUSY {busy}");
    }
    if !report.busy.is_empty() {
        let _ = writeln!(
            out,
            "  {} month(s) busy past {} ms: a writer (a pull) held them, so they were not compared; run this span again when it ends",
            report.busy.len(),
            zerodha_verify::MONTH_BUSY_RETRY.as_millis()
        );
    }
    if report.ledger_faults > 0 {
        let _ = writeln!(
            out,
            "  the gap ledger holds {} line(s) the verifier could not read",
            report.ledger_faults
        );
    }
    if report.stray > 0 {
        let _ = writeln!(
            out,
            "  {} stray name(s) in the store tree, never read",
            report.stray
        );
    }
    if report.stopped {
        let _ = writeln!(
            out,
            "  STOPPED before the span's end; what was spooled is merged below"
        );
    }
    // MONTHS HELD OUTSIDE THE 1MIN MONTHS ARE VERIFIED, AND SAID AS THAT
    // (IA-9 repair 1): a series held at 1day only, a 1day month before the
    // first 1min month.
    if report.beyond_months > 0 {
        let _ = writeln!(
            out,
            "  {} month(s) held outside their series' 1min months: verified at the rungs they hold (1day against its kept answer); no 1min record, as the store holds no minute there; daycheck named not run (daycheck_not_run)",
            report.beyond_months
        );
    }
    if report.unattested_days > 0 {
        let _ = writeln!(
            out,
            "  {} day(s) in those months that no calendar of record attests hold no bar and no kept answer: not judged (the calendar of record cannot say they traded)",
            report.unattested_days
        );
    }
    // A NAME THAT MATCHED NOTHING IS NOT A CLEAN RUN.
    for name in &report.unmatched {
        let _ = writeln!(
            out,
            "  NO SERIES {name}: no Zerodha series held under bars/zerodha answers to this name, so nothing of it was verified"
        );
    }
    if report.refused.is_empty()
        && report.busy.is_empty()
        && !report.stopped
        && report.unmatched.is_empty()
    {
        OK
    } else {
        FAILED
    }
}

/// Merges the run's spool in this process, under the merge lock; `false`
/// when the merge refused a spool or left one waiting. Another merger
/// holding the lock is said, and is not a failure of this run: its spool
/// waits for that merger or the next trigger.
fn merge_after(root: &Path, run: u64, out: &mut String) -> bool {
    // A MERGE IS A MERGE, IN WHICHEVER PROCESS: it keeps the merge's progress
    // file while it holds `verify/merge.lock`, as `cli verify-merge` does.
    let progress = pull::gdfl_run::ProgressFile::of_record(
        root.join(zerodha_verify::PROGRESS_MERGE),
        zerodha_verify::progress_record("verify-merge", 1, run, 1, None),
    );
    let merged = zerodha_verify::merge_observed(root, None, None, Some(&progress));
    crate::verify_merge_command::progress_said(&progress, out);
    match merged {
        Ok(outcome) => {
            let _ = writeln!(
                out,
                "VERIFY-MERGE (in this process, under {})",
                zerodha_verify::spool::MERGE_LOCK
            );
            render(&outcome, out);
            outcome.refused.is_empty()
                && outcome.faults.is_empty()
                && outcome.waiting_not_live() == 0
        }
        Err(MergeError::RunBusy { holder }) => {
            let _ = writeln!(
                out,
                "  run_busy: another merger holds {} ({holder}); the spool of run {run} waits in verify/spool/{run} for it (if it listed the spools while this run still spooled) or for the next verify-merge",
                zerodha_verify::spool::MERGE_LOCK
            );
            true
        }
        Err(why) => {
            let _ = writeln!(out, "  MERGE REFUSED: {why}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "the same exception every test module in this workspace takes"
    )]

    use super::*;

    #[test]
    fn zerodha_verify_takes_a_span_and_names() {
        let asked = ask(&[
            "2024-04-01",
            "2024-04-30",
            "NIFTY",
            "NSE-CASH-ABB",
            "--no-merge",
        ])
        .unwrap();
        assert_eq!(asked.request.only, ["NIFTY", "NSE-CASH-ABB"]);
        assert!(!asked.merge);
        assert!(ask(&["2024-04-01", "2024-04-30"]).unwrap().merge);
        // RA-145: `--workers N` names the width, 1 to 8; none takes the host's.
        assert_eq!(
            ask(&["2024-04-01", "2024-04-30", "--workers", "3", "NIFTY"])
                .unwrap()
                .workers,
            3
        );
        assert_eq!(
            ask(&["2024-04-01", "2024-04-30"]).unwrap().workers,
            zerodha_verify::cold_workers()
        );
        for wrong in [
            &["2024-04-01"][..],
            &["2024-04-30", "2024-04-01"],
            &["2024-4-1", "2024-04-30"],
            &["2024-04-01", "2024-04-31"],
            &["2024-04-01", "2024-04-30", "--no-merge", "--no-merge"],
            &["2024-04-01", "2024-04-30", "--merge"],
            &["2024-04-01", "2024-04-30", "../etc"],
            &["2024-04-01", "2024-04-30", "--workers"],
            &["2024-04-01", "2024-04-30", "--workers", "0"],
            &["2024-04-01", "2024-04-30", "--workers", "9"],
            &["2024-04-01", "2024-04-30", "--workers", "two"],
            &[
                "2024-04-01",
                "2024-04-30",
                "--workers",
                "2",
                "--workers",
                "2",
            ],
        ] {
            let mut out = String::new();
            assert_eq!(
                command_with(wrong, &|| panic!("never opened"), &mut out),
                MISUSED,
                "{wrong:?}"
            );
            assert!(out.contains("usage: cli zerodha-verify"), "{out}");
        }
    }

    #[test]
    fn zerodha_verify_on_an_empty_store_verifies_nothing_and_says_so() {
        let root = std::env::temp_dir().join(format!(
            "brutex-ia9-zv-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos())
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut out = String::new();
        let code = command_with(
            &["2024-04-01", "2024-04-30"],
            &|| Ok(root.clone()),
            &mut out,
        );
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains("0 series, 0 month(s), 0 day(s), 0 record(s)"),
            "{out}"
        );
        assert!(out.contains("spools merged"), "{out}");
        // A name no held series answers to is said, and the run is not OK
        // (IA-9 repair 1: before it, "0 series" and exit OK).
        let mut out = String::new();
        let code = command_with(
            &["2024-04-01", "2024-04-30", "ZZNOSUCH"],
            &|| Ok(root.clone()),
            &mut out,
        );
        assert_eq!(code, FAILED, "{out}");
        assert!(out.contains("NO SERIES ZZNOSUCH"), "{out}");
        // A second cold run while one holds the lock: run_busy.
        let file = pull::gdfl_run::locks::open_lock_file(&root, LOCK).unwrap();
        let lock = store::flock::Flock::try_lock(file, root.join(LOCK)).unwrap();
        let mut out = String::new();
        assert_eq!(
            command_with(
                &["2024-04-01", "2024-04-30"],
                &|| Ok(root.clone()),
                &mut out
            ),
            FAILED
        );
        assert!(out.contains("run_busy"), "{out}");
        assert!(out.contains("holds verify/zerodha.lock (held by "), "{out}");
        drop(lock);
    }

    /// A MONTH A PULL HELD IS SAID APART AND THE RUN IS NOT OK (design §6.3,
    /// "A month a writer holds"): its line reads `BUSY`, never `REFUSED`,
    /// with the wait it outlasted; and every unit the store lacks is shown
    /// by its class, minutes summed per (rung, verdict, reason) (DB-A12).
    /// A report with neither says neither and is OK.
    #[test]
    fn a_busy_month_is_said_apart_and_units_lacking_are_shown_by_class() {
        let asked = ask(&["2021-02-01", "2021-02-28", "NIFTY"]).unwrap();
        let reason = pull::verdict_words::reason("neither_vendor_has_it").unwrap();
        let report = zerodha_verify::ColdReport {
            series: 1,
            busy: vec!["NSE-INDEX-NIFTY 2021-02 5min: month_busy: a writer held it".to_owned()],
            missing: vec![(
                "1min".to_owned(),
                store::verdict::Code::VendorHole,
                reason,
                166,
            )],
            ..zerodha_verify::ColdReport::default()
        };
        let mut out = String::new();
        assert_eq!(
            render_report(7, &asked, &report, 1, &mut out),
            FAILED,
            "{out}"
        );
        assert!(
            out.contains("  BUSY NSE-INDEX-NIFTY 2021-02 5min: month_busy"),
            "{out}"
        );
        assert!(!out.contains("REFUSED"), "busy is not a refusal: {out}");
        assert!(out.contains("1 month(s) busy past 2000 ms"), "{out}");
        assert!(
            out.contains("vendor_hole (neither_vendor_has_it)") && out.contains("166"),
            "{out}"
        );
        let mut quiet = String::new();
        let clean = zerodha_verify::ColdReport {
            series: 1,
            ..zerodha_verify::ColdReport::default()
        };
        assert_eq!(
            render_report(7, &asked, &clean, 1, &mut quiet),
            OK,
            "{quiet}"
        );
        assert!(
            !quiet.contains("BUSY") && !quiet.contains("units"),
            "{quiet}"
        );
    }
}
