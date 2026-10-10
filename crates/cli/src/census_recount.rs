//! `cli census-recount` and `cli census-migrate` — the census verbs (D-3316,
//! WP-10).
//!
//! # `census-migrate VENDOR`
//!
//! Moves a vendor's single-file census (`manifest/<vendor>.man`, versions 1 to
//! 3) to version 4: epoch `000001` of one directory per segment under
//! `manifest/v4/<vendor>/`, each compacted to one row per key and carrying its
//! own capacity. The single file is left byte for byte as it was. Run it ONLY
//! once the API that serves the census reads version 4 (WP-30): an older reader
//! keeps reading the single file and would not see a month written after the
//! move (DB-G08). That ordering is WP-80 step 1 and the operator's to approve.
//!
//! # `census-recount VENDOR --from DAY [--to DAY]`
//!
//! Rebuilds census rows from the bar files' own headers for every month the
//! days overlap — O(files), no vendor file read (DB-G09) — and then closes the
//! span's census-refused GDFL import days with a `recounted` journal line each
//! (D-3317 item 5). This is how the options days refused by the full census
//! from 2019-11-14 are counted without re-reading a single zip (DB-G49,
//! DB-G60, DB-G78). `--from` is required: a span with no start would close
//! days the census never refused, such as the two vendor-refused days of
//! November 2018, and the operator's span is what excludes them.
//!
//! Both read and write only under the CLI's store root (`BRUTEX_STORE`), and
//! both exit [`FAILED`] when anything was refused or left open, so a script
//! cannot read a partial recount as a clean one.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::ingest::census_store::{self, Cycle, JournalClose, Location, Migration, Recount, Span};
use pull::session::Day;

use crate::{FAILED, MISUSED, OK};

/// Lines of a list a receipt prints before it only counts the rest.
const SHOWN: usize = 50;

/// A `YYYY-MM-DD` day.
fn day(text: &str) -> Result<Day, String> {
    let number = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u16>().ok())
            .flatten()
    };
    let parts: Vec<&str> = text.split('-').collect();
    let parsed = match parts.as_slice() {
        [y, m, d] => number(y, 4).zip(number(m, 2)).zip(number(d, 2)),
        _ => None,
    };
    let ((y, m), d) = parsed.ok_or_else(|| format!("{text:?} is not a YYYY-MM-DD day"))?;
    let (m, d) = (u8::try_from(m).unwrap_or(0), u8::try_from(d).unwrap_or(0));
    Day::new(y, m, d).map_err(|why| format!("{text:?}: {why}"))
}

/// A feed word, as the store names vendors.
fn vendor(word: &str) -> Result<Vendor, String> {
    Vendor::ALL
        .into_iter()
        .find(|vendor| vendor.as_str() == word)
        .ok_or_else(|| {
            let known: Vec<&str> = Vendor::ALL.iter().map(|vendor| vendor.as_str()).collect();
            format!(
                "`{}` is not a feed this build knows: {}",
                word.escape_debug(),
                known.join(", ")
            )
        })
}

/// `census-recount`'s arguments: the vendor, then `--from` (required) and
/// `--to` in any order, each at most once.
fn recount_arguments(arguments: &[&str]) -> Result<(Vendor, Span), String> {
    let [feed, flags @ ..] = arguments else {
        return Err("census-recount takes VENDOR --from DAY [--to DAY]".to_owned());
    };
    let vendor = vendor(feed)?;
    let mut from: Option<Day> = None;
    let mut to: Option<Day> = None;
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        let slot = match *flag {
            "--from" => &mut from,
            "--to" => &mut to,
            other => {
                return Err(format!(
                    "`{}` is not a census-recount flag; the flags are --from and --to",
                    other.escape_debug()
                ));
            }
        };
        if slot.is_some() {
            return Err(format!("{flag} is given twice"));
        }
        let Some(value) = rest.next() else {
            return Err(format!("{flag} needs a day: YYYY-MM-DD"));
        };
        *slot = Some(day(value)?);
    }
    let Some(from) = from else {
        return Err(
            "census-recount needs --from DAY: the span is what decides which census-refused \
             import days are closed, so it is never implied (D-3317)"
                .to_owned(),
        );
    };
    if let Some(to) = to
        && to < from
    {
        return Err(format!("--to {to} is before --from {from}"));
    }
    Ok((vendor, Span { from, to }))
}

/// `census-recount`, with the store root and the build's commit passed in so
/// every arm is driven by a test without the process environment.
pub(crate) fn recount_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    commit: Option<&str>,
    out: &mut String,
) -> u8 {
    let (vendor, span) = match recount_arguments(arguments) {
        Ok(parsed) => parsed,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    let found = store().and_then(|root| {
        let location = census_store::locate(&root, vendor)?;
        census_store::recount(&root, vendor, span, commit).map(|found| (root, location, found))
    });
    match found {
        Ok((root, location, found)) => {
            render_recount(out, vendor, span, (&root, &location), &found);
            if found.clean() { OK } else { FAILED }
        }
        Err(why) => {
            let _ = writeln!(out, "CENSUS RECOUNT REFUSED: {why}");
            FAILED
        }
    }
}

/// `census-recount` against the CLI's store root, stamped with the build's
/// commit.
pub(crate) fn recount(arguments: &[&str], out: &mut String) -> u8 {
    recount_with(arguments, &crate::store_root, crate::commit_stamp(), out)
}

/// `census-migrate`, with the store root passed in.
pub(crate) fn migrate_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let [feed] = arguments else {
        let _ = writeln!(out, "refused: census-migrate takes exactly VENDOR");
        return MISUSED;
    };
    let vendor = match vendor(feed) {
        Ok(vendor) => vendor,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    match store().and_then(|root| census_store::migrate(&root, vendor)) {
        Ok(done) => {
            render_migration(out, vendor, &done);
            OK
        }
        Err(why) => {
            let _ = writeln!(out, "CENSUS MIGRATE REFUSED: {why}");
            FAILED
        }
    }
}

/// `census-migrate` against the CLI's store root.
pub(crate) fn migrate(arguments: &[&str], out: &mut String) -> u8 {
    migrate_with(arguments, &crate::store_root, out)
}

/// The recount's receipt.
fn render_recount(
    out: &mut String,
    vendor: Vendor,
    span: Span,
    (root, location): (&Path, &Location),
    found: &Recount,
) {
    let to = span
        .to
        .map_or_else(|| "open".to_owned(), |to| to.to_string());
    let _ = writeln!(
        out,
        "CENSUS RECOUNT -- {} {} .. {to}, from bar-file headers under {} \
         (no vendor file was read)",
        vendor.as_str(),
        span.from,
        root.display()
    );
    let shape = match location {
        Location::Single(path) => format!("{} (single file, version 3)", path.display()),
        Location::Partitioned(dir) => {
            format!(
                "{} (one directory of epochs per segment, version 4)",
                dir.display()
            )
        }
    };
    let _ = writeln!(out, "  census              {shape}");
    let _ = writeln!(out, "  bar files seen      {}", found.files_seen);
    let _ = writeln!(out, "  in range            {}", found.files_in_range);
    let _ = writeln!(out, "  holding no bar      {}", found.files_empty);
    let _ = writeln!(out, "  unreadable          {}", found.unreadable);
    for (path, why) in &found.unreadable_named {
        let _ = writeln!(out, "    {}: {why}", path.display());
    }
    let _ = writeln!(out, "  not store names     {}", found.not_store_names);
    for path in &found.not_store_named {
        let _ = writeln!(out, "    {}", path.display());
    }
    let _ = writeln!(out, "  rows offered        {}", found.offered);
    if found.withdrawn > 0 {
        let _ = writeln!(
            out,
            "  rows withdrawn      {} (keys the gdfl re-key ledger names superseded or \
             retired; not counted back, D-3331)",
            found.withdrawn
        );
    }
    for cycle in &found.cycles {
        render_cycle(out, cycle);
    }
    if let Some(journal) = &found.journal {
        render_journal(out, journal);
    }
    if let Some(drift) = census_store::legacy_drift(root, vendor) {
        let _ = writeln!(out, "  WARNING             {drift}");
    }
}

/// One census's line of the receipt.
fn render_cycle(out: &mut String, cycle: &Cycle) {
    let epoch = cycle
        .epoch
        .map_or_else(String::new, |epoch| format!(" (epoch {epoch:06})"));
    let _ = writeln!(
        out,
        "  {}{epoch}: recorded {}, already exact {}, refused {}; now generation {}, \
         {} entries, {} keys, {} rows, capacity {}",
        cycle.path.display(),
        cycle.recorded,
        cycle.exact,
        cycle.refused.len(),
        cycle.generation,
        cycle.entries,
        cycle.keys,
        cycle.rows,
        cycle.capacity
    );
    if let Some(done) = &cycle.compacted {
        let from = done
            .from_epoch
            .map_or_else(|| "none".to_owned(), |epoch| format!("{epoch:06}"));
        let _ = writeln!(
            out,
            "    compacted ({}) epoch {from} -> {:06}: {} -> {} entries; epoch {from} is kept \
             as it was",
            done.why, done.to_epoch, done.from_entries, done.to_entries
        );
    }
    if let Some(why) = &cycle.degraded {
        let _ = writeln!(
            out,
            "    loaded DEGRADED, and this write was the repair: {why}"
        );
    }
    for (key, why) in cycle.refused.iter().take(SHOWN) {
        let contract = key
            .contract
            .as_ref()
            .map_or_else(String::new, |contract| format!(" {}", contract.as_str()));
        let _ = writeln!(
            out,
            "    refused {} {}{contract} {} {}: {why}",
            key.segment.as_str(),
            key.symbol.as_str(),
            key.timeframe.as_str(),
            key.month
        );
    }
}

/// The journal closing's lines of the receipt.
fn render_journal(out: &mut String, journal: &JournalClose) {
    let run = journal
        .run
        .map_or_else(|| "no line written".to_owned(), |run| format!("run {run}"));
    let _ = writeln!(
        out,
        "  import journal      {} ({run})",
        journal.path.display()
    );
    let first = journal.closed.first().map(|day| day.key.as_str());
    let last = journal.closed.last().map(|day| day.key.as_str());
    let _ = writeln!(
        out,
        "  days recounted      {}{}",
        journal.closed.len(),
        match (first, last) {
            (Some(first), Some(last)) => format!(" ({first} .. {last})"),
            _ => String::new(),
        }
    );
    // `files=` counts every file the import converted, including one that
    // wrote no bar that day, so a closed day can have more files than months
    // spanning it. Said, never hidden, and never a reason to hold the day.
    let beyond = journal
        .closed
        .iter()
        .filter(|day| day.files.is_some_and(|files| files > day.months))
        .count();
    let _ = writeln!(
        out,
        "  files beyond months {beyond} closed day(s) converted more files than the months \
         spanning them (a file that wrote no bar that day spans none)"
    );
    let _ = writeln!(out, "  days left open      {}", journal.left_open.len());
    for (key, why) in journal.left_open.iter().take(SHOWN) {
        let _ = writeln!(out, "    {key}: {why}");
    }
    let _ = writeln!(
        out,
        "  other-stage days    {} (closed incomplete at a stage that is not the census; \
         a recount never closes them)",
        journal.other_stage.len()
    );
    for key in journal.other_stage.iter().take(SHOWN) {
        let _ = writeln!(out, "    {key}");
    }
}

/// The migration's receipt.
fn render_migration(out: &mut String, vendor: Vendor, done: &Migration) {
    let (version, generation, entries, keys, rows) = done.legacy_counters;
    let _ = writeln!(
        out,
        "CENSUS MIGRATED -- {} to version 4, epoch 000001 of each segment (D-3316)",
        vendor.as_str()
    );
    if done.legacy_present {
        let _ = writeln!(
            out,
            "  single file         {}: version {version}, generation {generation}, \
             {entries} entries, {keys} keys, {rows} rows, {} bytes -- left exactly as it was",
            done.legacy.display(),
            done.legacy_bytes
        );
    } else {
        let _ = writeln!(
            out,
            "  single file         {} did not exist: the vendor starts at version 4, empty",
            done.legacy.display()
        );
    }
    let _ = writeln!(out, "  partitions          {}", done.dir.display());
    for part in &done.partitions {
        let _ = writeln!(
            out,
            "    {:<5} {} keys, {} rows, capacity {}, {} bytes -- {}",
            part.segment.as_str(),
            part.keys,
            part.rows,
            part.capacity,
            part.bytes,
            part.path.display()
        );
    }
    let _ = writeln!(
        out,
        "  readers             a census reader built before version 4 reads only \
         the single file; serve the store with a version-4-aware build (WP-30)"
    );
}

#[cfg(test)]
#[path = "census_recount_tests.rs"]
mod tests;
