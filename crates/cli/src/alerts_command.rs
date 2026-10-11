//! `cli alerts [--ack <n>] [--older] [--before <n>] [--replace-damaged]
//! [--file <path>]`: the alert queue from a terminal (D-4883, D-4893, D-4895).
//!
//! With no flag it prints what every page shows: the unacknowledged alerts
//! among the newest 1,024 records of `<store>/audit/alerts`, newest first,
//! the exact count of older unacknowledged ones, and any record that fails its
//! CRC, in one read (`pull::alerts::window`).
//!
//! The other flags are the operator's steps a route never takes
//! (`docs/06-limits.md` L-IA-34): `--before <n>` prints the page before
//! ordinal `n` as it stands now, which is one read of that page and then the
//! records after it for their acknowledgements (`pull::alerts::page_settled`),
//! so no alert a newer record acknowledges is listed and its counts agree with
//! `--older`; `--older` reads the whole queue backwards, `--ack <n>`
//! acknowledges an alert inside or outside the window, `--replace-damaged`
//! keeps a damaged queue and puts a new one in its place, and `--file <path>`
//! lists a kept file.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pull::alerts::{self, Alert, Kept, Page, Record, Scope};

use crate::{FAILED, MISUSED, OK};

/// The one line a misuse prints.
const USAGE: &str =
    "usage: cli alerts [--ack <n> | --older | --before <n> | --replace-damaged | --file <path>]";

/// What was asked.
#[derive(Debug, PartialEq, Eq)]
enum Ask<'a> {
    /// A step on the store's own queue.
    Queue(Step),
    /// List a kept file, which needs no store root.
    File(&'a str),
}

/// A step on `<store>/audit/alerts`.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    /// The newest page, or the one before an ordinal.
    List(Option<u32>),
    /// Every unacknowledged alert older than the newest page.
    Older,
    /// Acknowledge one alert.
    Ack(u32),
    /// Keep a damaged queue and start a new one.
    Replace,
}

/// The arguments as an [`Ask`], or why they are a misuse.
fn parse<'a>(arguments: &[&'a str]) -> Result<Ask<'a>, String> {
    let ordinal = |flag: &str, text: &str| {
        text.parse::<u32>()
            .map_err(|_| format!("{flag} takes an alert's ordinal, not {text:?}"))
    };
    match arguments {
        [] => Ok(Ask::Queue(Step::List(None))),
        ["--older"] => Ok(Ask::Queue(Step::Older)),
        ["--replace-damaged"] => Ok(Ask::Queue(Step::Replace)),
        ["--ack", n] => ordinal("--ack", n).map(|n| Ask::Queue(Step::Ack(n))),
        ["--before", n] => ordinal("--before", n).map(|n| Ask::Queue(Step::List(Some(n)))),
        ["--file", path] if !path.is_empty() => Ok(Ask::File(path)),
        other => Err(format!("{other:?} is not a form of this verb")),
    }
}

/// `YYYY-MM-DD` of days since 1970-01-01.
fn day_text(days: u32) -> String {
    let (y, m, d) = telemetry::civil_from_days(i64::from(days));
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-MM-DDTHH:MM:SSZ` of unix seconds.
fn moment_text(unix_s: u64) -> String {
    let days = u32::try_from(unix_s / 86_400).unwrap_or(u32::MAX);
    let rest = unix_s % 86_400;
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        day_text(days),
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

/// One record's line: an alert, an acknowledgement, or a later build's type.
fn record_line(out: &mut String, ordinal: u32, record: &Record) {
    let when = moment_text(record.put_at);
    match record.kind_of_record {
        alerts::TYPE_ALERT => {
            let day = if record.day == 0 {
                "-".to_owned()
            } else {
                day_text(record.day)
            };
            let cut = if record.text_cut() { " [text cut]" } else { "" };
            let _ = writeln!(
                out,
                "  #{ordinal}  {}  run {}  day {day}  put {when}  {}{cut}",
                alerts::kind_label(record.kind),
                record.run,
                record.text
            );
        }
        alerts::TYPE_ACK => {
            let _ = writeln!(
                out,
                "  #{ordinal}  acknowledges #{}  by {}  at {when}",
                record.acked, record.text
            );
        }
        other => {
            let _ = writeln!(
                out,
                "  #{ordinal}  UNKNOWN record type {other} (a later build's), put {when}"
            );
        }
    }
}

fn alert_lines(out: &mut String, alerts: &[Alert]) {
    for alert in alerts {
        record_line(out, alert.ordinal, &alert.record);
    }
}

/// The page every reader shows.
fn render_page(out: &mut String, store: &Path, before: Option<u32>, page: &Page) -> u8 {
    let queue = alerts::Names::under(store).queue;
    if !page.exists {
        let _ = writeln!(out, "ALERTS -- {}: no queue yet, no alert", queue.display());
        return OK;
    }
    let (which, older) = before.map_or_else(
        || {
            (
                format!("the newest {} records", alerts::WINDOW),
                "older unacknowledged (cli alerts --older)",
            )
        },
        |n| {
            (
                format!("the {} records before #{n}", alerts::WINDOW),
                "unacknowledged older than this page",
            )
        },
    );
    let _ = writeln!(
        out,
        "ALERTS -- {}: {} unacknowledged in {which}; {} {older}; {} records",
        queue.display(),
        page.alerts.len(),
        page.older_unacknowledged,
        page.records
    );
    alert_lines(out, &page.alerts);
    for (ordinal, kind) in &page.unknown {
        let _ = writeln!(
            out,
            "  #{ordinal}  UNKNOWN record type {kind} (a later build's)"
        );
    }
    for ordinal in &page.damaged {
        let _ = writeln!(
            out,
            "  #{ordinal}  DAMAGED: the record fails its CRC; the older count includes it"
        );
    }
    if page.torn > 0 {
        let _ = writeln!(
            out,
            "  {} bytes of a torn append follow the last whole record; the next put writes over them",
            page.torn
        );
    }
    if page.later_damaged > 0 {
        let _ = writeln!(
            out,
            "  {} records after this page fail their CRC: an acknowledgement among them could \
             not be applied; cli alerts names them",
            page.later_damaged
        );
    }
    if page.first > 1 {
        let _ = writeln!(out, "  the page before: cli alerts --before {}", page.first);
    }
    if page.damaged.is_empty() && page.later_damaged == 0 {
        OK
    } else {
        FAILED
    }
}

/// `--file`: every record of a kept file.
fn render_file(out: &mut String, path: &Path) -> u8 {
    let mut lines = String::new();
    let mut damaged = 0_u64;
    let listed = alerts::of_file(path, &mut |kept| match kept {
        Kept::Good(ordinal, record) => record_line(&mut lines, ordinal, &record),
        Kept::Damaged(ordinal) => {
            damaged += 1;
            let _ = writeln!(lines, "  #{ordinal}  DAMAGED: the record fails its CRC");
        }
    });
    match listed {
        Ok(shape) => {
            let _ = writeln!(
                out,
                "ALERTS FILE -- {}: {} records, {damaged} damaged, header {}, {} torn bytes",
                path.display(),
                shape.records,
                if shape.header_ok { "ok" } else { "DAMAGED" },
                shape.torn
            );
            out.push_str(&lines);
            OK
        }
        Err(why) => {
            let _ = writeln!(out, "ALERTS REFUSED: {why}");
            FAILED
        }
    }
}

/// The verb, with its store root passed in so every arm is driven by a test
/// without touching the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let ask = match parse(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "ALERTS MISUSED: {why}\n{USAGE}");
            return MISUSED;
        }
    };
    let step = match ask {
        Ask::File(path) => return render_file(out, Path::new(path)),
        Ask::Queue(step) => step,
    };
    let store = match store() {
        Ok(store) => store,
        Err(why) => {
            let _ = writeln!(out, "ALERTS REFUSED: {why}");
            return FAILED;
        }
    };
    let done = match step {
        // The newest page is whole in one read. An older one is listed as it
        // stands now: the records after it are read for their
        // acknowledgements, so every form of this verb agrees (L-IA-34).
        Step::List(before) => before
            .map_or_else(
                || alerts::window(&store),
                |n| alerts::page_settled(&store, n),
            )
            .map(|page| render_page(out, &store, before, &page)),
        Step::Older => alerts::older(&store).map(|older| {
            let _ = writeln!(
                out,
                "ALERTS OLDER -- {} unacknowledged older than the newest {} records",
                older.len(),
                alerts::WINDOW
            );
            alert_lines(out, &older);
            OK
        }),
        Step::Ack(ordinal) => alerts::ack(&store, ordinal, "cli", Scope::Whole).map(|acked| {
            let _ = writeln!(
                out,
                "ALERT ACKNOWLEDGED: #{} ({}, run {}); recorded as #{}; unacknowledged left: {}",
                acked.ordinal,
                alerts::kind_label(acked.kind),
                acked.run,
                acked.record,
                acked.open_after
            );
            OK
        }),
        Step::Replace => alerts::replace_damaged(&store).map(|replaced| {
            let held = replaced
                .open
                .map_or_else(|| "unknown".to_owned(), |n| n.to_string());
            let _ = writeln!(
                out,
                "ALERT QUEUE REPLACED: kept {} (never edited, never deleted); unacknowledged \
                 alerts it held: {held}; the new queue's alert #{} says so; list the kept file \
                 with: cli alerts --file {}",
                replaced.kept.display(),
                replaced.ordinal,
                replaced.kept.display()
            );
            OK
        }),
    };
    done.unwrap_or_else(|why| {
        let _ = writeln!(out, "ALERTS REFUSED: {why}");
        FAILED
    })
}

/// The CLI's store root, proved as every other verb proves it.
fn cli_store() -> Result<PathBuf, String> {
    crate::store_root()
}

/// The verb as dispatched.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(arguments, &cli_store, out)
}

#[cfg(test)]
#[path = "alerts_command_tests.rs"]
mod tests;
