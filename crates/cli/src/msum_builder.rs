//! `cli msum-build` — each named month's `.sum` month summary written whole
//! through the store's rebuild door, or confirmed, as one run. R3-06, D-4980;
//! DB-A30.
//!
//! # Usage
//!
//! ```text
//! cli msum-build VENDOR EXCHANGE/SEGMENT/SYMBOL[/CONTRACT|/*] RUNG FROM TO [--dry-run]
//! cli msum-build gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09
//! cli msum-build zerodha NSE/INDEX/NIFTY 1min 2018-01 2026-09
//! cli msum-build gdfl NSE/FNO/NIFTY/* 1s 2018-09 2020-09 --dry-run
//! ```
//!
//! The arguments are `cli srt-build`'s, parsed by the same function
//! ([`crate::srt_builder::ask_for`]), so the two verbs name a series and a
//! span the same way and refuse the same misuse before anything is opened.
//!
//! # Why it exists
//!
//! `/gaps.json` reads one month summary a month asked and no bar when the
//! month keeps one (DB-A30). A writer keeps every month it writes summarised
//! from D-4980 on; the months written before it have no `.sum` until a writer
//! opens them again, which an import that is done with a month never does, so
//! their gap answers read every bar and say `built`. This verb is that step
//! for named months, and nothing else, through
//! `store::file::BarFile::month_summary_existing`:
//!
//! * a month that is not held is reported and NOTHING is created for it;
//! * the bar file, its checksums and its indexes are opened read-only and
//!   never change;
//! * a month another process holds — a writer, or a reader mid-request — is
//!   refused by the store's own exclusive lock, named, and left as it was;
//! * a month whose summary already describes its committed header is kept, so
//!   a rerun is safe and resumes where one stopped;
//! * anything else is written whole into a NEW file renamed over the old name
//!   — never edited in place — and the store writes one `store.sum` line
//!   naming the month and why;
//! * each month is then reopened through the READ side, and the summary a
//!   reader finds is checked to describe the committed header.
//!
//! # The small-month rule (D-4933)
//!
//! A month under 1,024 bars (`store::time_index::INDEX_MIN_ROWS`) with no
//! `.sum` is given none, by the verb as by the writer: it is reported `small`,
//! nothing is written, and it counts as clean, because a reader builds the
//! summary of its at most 1,023 bars itself.
//!
//! `--dry-run` opens every month read-only, says which keep a summary and
//! which would be built, and writes nothing. A month whose `.sum` name or
//! `.new.sum` staging name is there and is not a regular file is `REFUSED` by
//! the dry run as the store's door refuses it.
//!
//! # Cost
//!
//! Building one month reads every committed bar once through the verified
//! read path and folds each in O(1); the file written is at most
//! `store::month_summary::MAX_PAYLOAD_LEN` bytes plus 16 (1.5 KB for a minute
//! month, tens of KB for a one-second index month). The verb holds one
//! month's lock at a time.
//!
//! # Exit
//!
//! [`OK`] only when every month in the span is held and a reader finds its
//! summary (or it is small or empty; with `--dry-run`, held and readable);
//! [`FAILED`] otherwise, so a script cannot read a partial build — or a `*`
//! that found nothing — as a clean one; [`MISUSED`] for arguments that do not
//! name a series and a span, before anything is opened.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use store::file::{BarFile, StoreError, Summarized};
use store::month_summary::{Refused, path_beside, read_beside};
use store::path::StorePath;
use store::time_index::INDEX_MIN_ROWS;

use crate::srt_builder::{Which, ask_for};
use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli msum-build VENDOR EXCHANGE/SEGMENT/SYMBOL[/CONTRACT|/*] RUNG \
     FROM TO [--dry-run]  (FROM and TO are YYYY-MM, inclusive; e.g. gdfl NSE/INDEX/NIFTY 1s \
     2018-09 2026-09)";

/// One month's line in the receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    /// The summary already described the committed header: nothing written.
    Kept { n_valid: u64 },
    /// Written whole, for the reason the store named.
    Built { n_valid: u64, because: String },
    /// No committed row: nothing to summarise.
    Empty,
    /// Under the small-month threshold with no `.sum`: none is made.
    Small { n_valid: u64 },
    /// No such month: nothing was created.
    NotHeld,
    /// The store refused the month; nothing was written to it.
    Refused { why: String },
    /// The step finished and a reader still refuses the summary it finds.
    NotSummarized { n_valid: u64, reader: String },
    /// Dry run: a reader finds a summary of these bars.
    Summarized { n_valid: u64 },
    /// Dry run: the step would write the summary whole, and why.
    WouldBuild { n_valid: u64, why: String },
}

impl Line {
    /// Whether this month ends the run summarised (or, dry, held and
    /// readable).
    const fn clean(&self) -> bool {
        matches!(
            self,
            Self::Kept { .. }
                | Self::Built { .. }
                | Self::Empty
                | Self::Small { .. }
                | Self::Summarized { .. }
                | Self::WouldBuild { .. }
        )
    }
}

/// How a reader sees the month: its committed rows, and the summary it finds
/// or why it finds none. Writes nothing.
fn reader_view(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
) -> Result<(u64, PathBuf, Result<(), Refused>), StoreError> {
    // THE GENERATION EVERY READER OPENS (D-3350): the `.sum` a reader reads
    // sits beside the bar file it opened.
    let file = BarFile::open_existing(root, path, symbol_id)?;
    let header = file.header();
    let found = read_beside(file.path(), &header, path.month()).map(|_| ());
    Ok((header.n_valid, path_beside(file.path()), found))
}

/// One month, built or confirmed, then checked from the read side.
fn build(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Line {
    let done = match BarFile::month_summary_existing(root, path, symbol_id) {
        Ok(done) => done,
        Err(StoreError::Missing { .. }) => return Line::NotHeld,
        Err(StoreError::Locked { path }) => {
            return Line::Refused {
                why: format!(
                    "{} is held by another process (a writer, or a reader mid-request) and is \
                     not waited for",
                    path.display()
                ),
            };
        }
        Err(why) => {
            return Line::Refused {
                why: why.to_string(),
            };
        }
    };
    let because = match done {
        Summarized::Kept => None,
        Summarized::Built(because) => Some(because),
        Summarized::Empty => return Line::Empty,
        Summarized::BelowThreshold => match reader_view(root, path, symbol_id) {
            Ok((n_valid, _, _)) => return Line::Small { n_valid },
            Err(why) => {
                return Line::Refused {
                    why: format!("the step finished and the read-back was refused: {why}"),
                };
            }
        },
        other => {
            return Line::Refused {
                why: format!("the store reported an outcome this build does not know: {other:?}"),
            };
        }
    };
    match reader_view(root, path, symbol_id) {
        Ok((n_valid, _, Ok(()))) => because.map_or(Line::Kept { n_valid }, |because| Line::Built {
            n_valid,
            because,
        }),
        Ok((n_valid, _, Err(reader))) => Line::NotSummarized {
            n_valid,
            reader: reader.to_string(),
        },
        Err(why) => Line::Refused {
            why: format!("the step finished and the read-back was refused: {why}"),
        },
    }
}

/// Why the build step would refuse this month for what sits at `name` — the
/// month's `.sum` or its staging name — when that is there and is not a
/// regular file: a link, a directory, a pipe. The store's door leaves such a
/// name as it is and refuses the month, so a dry run says the same.
fn name_refused(name: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(name).ok()?;
    (!meta.is_file()).then(|| {
        format!(
            "{} is not a regular file; the build step leaves it as it is and refuses the month",
            name.display()
        )
    })
}

/// One month as a reader sees it, writing nothing — and saying what the build
/// step WOULD do, never something else.
fn inspect(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Line {
    match reader_view(root, path, symbol_id) {
        Ok((0, _, _)) => Line::Empty,
        Ok((n_valid, sum, found)) => {
            let staging = sum.with_extension("new.sum");
            match (name_refused(&sum).or_else(|| name_refused(&staging)), found) {
                (Some(why), _) => Line::Refused { why },
                (None, Ok(())) => Line::Summarized { n_valid },
                (None, Err(Refused::BelowThreshold)) => Line::Small { n_valid },
                (None, Err(why)) => Line::WouldBuild {
                    n_valid,
                    why: why.to_string(),
                },
            }
        }
        Err(StoreError::Missing { .. }) => Line::NotHeld,
        Err(why) => Line::Refused {
            why: why.to_string(),
        },
    }
}

/// The receipt line for one month.
fn render_line(out: &mut String, label: &str, line: &Line) {
    let _ = match line {
        Line::Kept { n_valid } => writeln!(
            out,
            "  {label}  kept            {n_valid:>9} bars  its summary describes these bars; nothing written"
        ),
        Line::Built { n_valid, because } => writeln!(
            out,
            "  {label}  built           {n_valid:>9} bars  because {because}; a reader now finds its summary"
        ),
        Line::Empty => writeln!(
            out,
            "  {label}  empty           no committed bar to summarise"
        ),
        Line::Small { n_valid } => writeln!(
            out,
            "  {label}  small           {n_valid:>9} bars  under {INDEX_MIN_ROWS} bars: no .sum by the small-month rule (D-4933); a reader summarises these rows itself; nothing written"
        ),
        Line::NotHeld => writeln!(
            out,
            "  {label}  NOT HELD        no such month; nothing was created"
        ),
        Line::Refused { why } => writeln!(
            out,
            "  {label}  REFUSED         {why}; nothing was written to it"
        ),
        Line::NotSummarized { n_valid, reader } => writeln!(
            out,
            "  {label}  NOT SUMMARIZED  {n_valid:>9} bars  the step finished and a reader finds {reader}"
        ),
        Line::Summarized { n_valid } => {
            writeln!(out, "  {label}  summarized      {n_valid:>9} bars")
        }
        Line::WouldBuild { n_valid, why } => {
            writeln!(out, "  {label}  would build     {n_valid:>9} bars  {why}")
        }
    };
}

/// Runs the verb against the store root `store` names.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let ask = match ask_for("msum-build", arguments) {
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
                "MSUM BUILD REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    let walk = match ask.series(&root) {
        Ok(walk) => walk,
        Err(why) => {
            let _ = writeln!(out, "MSUM BUILD REFUSED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    let symbol_id = ask.symbol_id();
    let span = |month: Option<&store::path::YearMonth>| month.map(ToString::to_string);
    let (first, last) = (
        span(ask.months.first()).unwrap_or_default(),
        span(ask.months.last()).unwrap_or_default(),
    );
    let _ = writeln!(
        out,
        "MSUM BUILD{} — {} {}/{}/{} {}, {first}..{last}: {} series × {} month(s) under {}",
        if ask.dry_run {
            " (dry run: nothing is written)"
        } else {
            ""
        },
        ask.vendor.as_str(),
        ask.exchange,
        ask.segment,
        ask.symbol,
        ask.rung.as_str(),
        walk.series.len(),
        ask.months.len(),
        root.display()
    );
    // WHAT THE WALK LEFT ALONE IS SAID, never skipped silently.
    if walk.links > 0 || walk.others > 0 {
        let _ = writeln!(
            out,
            "  NOT WALKED: {} symbolic link(s) — never followed, nothing is built through a \
             link — and {} other entr(ies) that are not contract directories",
            walk.links, walk.others
        );
    }
    let mut receipt = Vec::with_capacity(
        walk.series
            .len()
            .saturating_mul(ask.months.len())
            .min(4_096),
    );
    for &contract in &walk.series {
        let name = contract.map_or_else(String::new, |contract| format!("{contract} "));
        for &month in &ask.months {
            let line = match ask.path(contract, month) {
                Ok(path) if ask.dry_run => inspect(&root, path, symbol_id),
                Ok(path) => build(&root, path, symbol_id),
                Err(why) => Line::Refused { why },
            };
            // A `*` WALK NAMES ONLY WHAT IT FOUND: a contract's life is a few
            // months, so the others are not held by design.
            if !(ask.which == Which::Every && line == Line::NotHeld) {
                render_line(out, &format!("{name}{month}"), &line);
                receipt.push(line);
            }
        }
    }
    if receipt.is_empty() {
        let _ = writeln!(
            out,
            "NO MONTH HELD: none of the {} contract(s) holds a month in {first}..{last}. \
             Nothing was summarised; nothing was written.",
            walk.series.len()
        );
        return FAILED;
    }
    summed(out, &receipt, ask.dry_run)
}

/// The receipt's last line and the exit: [`OK`] only when every month named
/// ended summarised (or, dry, held and readable).
fn summed(out: &mut String, receipt: &[Line], dry_run: bool) -> u8 {
    let count = |want: fn(&Line) -> bool| receipt.iter().filter(|line| want(line)).count();
    let clean = receipt.iter().all(Line::clean);
    if dry_run {
        let _ = writeln!(
            out,
            "Summarized {}, would build {}, empty {}, small {}, not held {}, refused {}. Nothing was written.",
            count(|line| matches!(line, Line::Summarized { .. })),
            count(|line| matches!(line, Line::WouldBuild { .. })),
            count(|line| matches!(line, Line::Empty)),
            count(|line| matches!(line, Line::Small { .. })),
            count(|line| matches!(line, Line::NotHeld)),
            count(|line| matches!(line, Line::Refused { .. })),
        );
    } else {
        let _ = writeln!(
            out,
            "Built {}, kept {}, empty {}, small {}, not summarized {}, not held {}, refused {}: {}",
            count(|line| matches!(line, Line::Built { .. })),
            count(|line| matches!(line, Line::Kept { .. })),
            count(|line| matches!(line, Line::Empty)),
            count(|line| matches!(line, Line::Small { .. })),
            count(|line| matches!(line, Line::NotSummarized { .. })),
            count(|line| matches!(line, Line::NotHeld)),
            count(|line| matches!(line, Line::Refused { .. })),
            if clean {
                "every month in the span keeps a summary, or holds too few bars to need one (D-4933)"
            } else {
                "NOT every month in the span keeps a summary; rerun once the cause is gone (a rerun keeps what is built)"
            }
        );
    }
    if clean { OK } else { FAILED }
}

/// One month built as an item of an upkeep run (R3-17, D-4997): the step
/// this verb takes for it, and the receipt line this verb prints for it as the
/// run's outcome, so the two never say different things.
pub(crate) fn upkeep_month(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
) -> pull::upkeep::Outcome {
    let line = build(root, path, symbol_id);
    let label = path.month().to_string();
    let mut text = String::new();
    render_line(&mut text, &label, &line);
    crate::upkeep_command::outcome(line.clean(), &label, &text)
}

/// `cli msum-build`, against the CLI's store root (`BRUTEX_STORE`).
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // AN UNSTAMPED BUILD WRITES NO SUMMARY (DB-B71, WP-81): a `.sum` is a file
    // of the store that `/gaps.json` answers from. `--dry-run` writes
    // nothing and is not refused.
    let store = crate::store_write::root_for("msum-build", arguments);
    command_with(arguments, &store, out)
}

#[cfg(test)]
#[path = "msum_builder_tests.rs"]
mod tests;
