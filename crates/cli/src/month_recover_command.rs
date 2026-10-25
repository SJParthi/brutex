//! `cli month-recover` — the store door that brings back a month a power cut
//! left refused, run on the operator's yes. R3-20, D-7093; NEW-23, RA-276,
//! RA-280, RA-330, FM-062.
//!
//! # Usage
//!
//! ```text
//! cli month-recover VENDOR EXCHANGE/SEGMENT/SYMBOL[/CONTRACT] RUNG YYYY-MM --plan
//! cli month-recover VENDOR EXCHANGE/SEGMENT/SYMBOL[/CONTRACT] RUNG YYYY-MM --yes
//! cli month-recover gdfl NSE/INDEX/NIFTY 1s 2020-03 --plan
//! ```
//!
//! `--plan` writes nothing: it asks `store::recover::plan` whether the month
//! is clean, already recovered, or refused, and for a refused one names the
//! month's own refusal, every commit tried and why, the cut (the last commit
//! whose records check out, or the whole blocks that do), what a surviving
//! slot published, the bars and IST days the cut drops, and which of those
//! days the import journal shows done.
//!
//! `--yes` is the operator's yes, and the only form that writes:
//!
//! 1. `store::recover::recover` writes the kept records as a NEW revision of
//!    the month and makes it the served generation (the original's bytes are
//!    never changed, truncated or renamed), and records the recovery;
//! 2. the importer is told to read the dropped days again: for a GDFL month,
//!    each dropped day the import journal shows `done` gets one `incomplete`
//!    line in the journal's own grammar, with the reason `store_damaged`,
//!    which `admit` reads on a plain Resume (it is no hole reason); a day
//!    the journal does not show done is read again by any Resume already. A
//!    month of another feed names its days for that feed's re-pull;
//! 3. one line in `audit/<vendor>.recoveries` names the month, the cut, the
//!    revision, the dropped bars and days and the journal lines written: the
//!    run's end record;
//! 4. `store::recover::note_reread` marks the recovery's record, so a run
//!    cut off before it writes steps 2 and 3 again, and a run after it
//!    writes nothing and says so.
//!
//! It never runs by itself: it is a step of the store-step list
//! (`config/store-steps.toml`), run on the operator's yes. There is no
//! button, no route and no recovery at start (D-6900).
//!
//! # Exit
//!
//! [`OK`] when the plan could be made (clean, recovered or cut), and for
//! `--yes` when the month is clean, already recovered or recovered with its
//! re-read named; [`FAILED`] for every refusal, said by name, and for a
//! journal or ledger step that could not be written (the recovery stands and
//! the same command writes what is owed); [`MISUSED`] for arguments it does
//! not know, before anything is opened.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use pull::gdfl_import::ImportKind;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::recover::{Dropped, Outcome, Plan, Record, RecoverError};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli month-recover VENDOR EXCHANGE/SEGMENT/SYMBOL[/CONTRACT] RUNG \
     YYYY-MM --plan|--yes  (--plan writes nothing; --yes is the operator's yes and writes the \
     month's recovered revision; e.g. gdfl NSE/INDEX/NIFTY 1s 2020-03 --plan)";

/// The telemetry target of the verb's own events: a lifecycle, so the audit
/// stream and the state stream keep them (`telemetry::is_audited`).
const TARGET: &str = "cli.month_recover.lifecycle";

/// The audit ledger the run's end record goes to: `audit/<vendor>.recoveries`.
pub(crate) const LEDGER: &str = "recoveries";

/// The reason the read-again lines carry: the month refused for a fault of
/// its own, the importer's own word for it, which `admit` reads on a plain
/// Resume (it is not one of the hole reasons).
pub(crate) const REREAD_REASON: &str = "store_damaged";

/// What the arguments name: one month of one series, and the form.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    vendor: Vendor,
    exchange: String,
    segment: String,
    symbol: String,
    contract: Option<Contract>,
    rung: Timeframe,
    month: YearMonth,
    write: bool,
}

impl Ask {
    /// The month's bar path.
    fn path(&self) -> Result<StorePath<'_>, String> {
        StorePath::new(PathParts {
            vendor: self.vendor,
            exchange: &self.exchange,
            segment: &self.segment,
            symbol: &self.symbol,
            contract: self.contract,
            timeframe: self.rung,
            month: self.month,
            file: FileKind::Bars,
        })
        .map_err(|why| format!("{} is not a store path: {why}", self.named()))
    }

    /// The header's symbol id, derived as every writer derives it: the low
    /// 32 bits of the underlying symbol's FNV-1a (`pull::ingest`'s
    /// `identify`), the same for the index and every contract of it.
    fn symbol_id(&self) -> u32 {
        let [a, b, c, d, ..] = brutex_core::universe::fnv1a(&self.symbol).to_le_bytes();
        u32::from_le_bytes([a, b, c, d])
    }

    /// The month as the operator typed it.
    fn named(&self) -> String {
        let contract = self
            .contract
            .map_or_else(String::new, |contract| format!("/{contract}"));
        format!(
            "{} {}/{}/{}{contract} {} {}",
            self.vendor.as_str(),
            crate::stored::clipped(&self.exchange),
            crate::stored::clipped(&self.segment),
            crate::stored::clipped(&self.symbol),
            self.rung.as_str(),
            self.month
        )
    }

    /// The IST date of day `day` of the month, `YYYY-MM-DD`.
    fn date(&self, day: u8) -> String {
        format!("{}-{day:02}", self.month)
    }
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
                crate::stored::clipped(word),
                known.join(", ")
            )
        })
}

/// The arguments, refused by name before anything is opened.
fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let [feed, instrument, rung, month, form] = arguments else {
        return Err(format!(
            "month-recover takes five arguments, the last --plan or --yes; it was given {}",
            arguments.len()
        ));
    };
    let write = match *form {
        "--plan" => false,
        "--yes" => true,
        other => {
            return Err(format!(
                "`{}` is not --plan or --yes: the plan writes nothing, and --yes is the \
                 operator's yes to write the recovered month",
                crate::stored::clipped(other)
            ));
        }
    };
    let parts: Vec<&str> = instrument.split('/').collect();
    let (exchange, segment, symbol, contract) = match parts.as_slice() {
        [exchange, segment, symbol] => (*exchange, *segment, *symbol, None),
        [exchange, segment, symbol, contract] => (*exchange, *segment, *symbol, Some(*contract)),
        _ => {
            return Err(format!(
                "`{}` is not EXCHANGE/SEGMENT/SYMBOL[/CONTRACT], the store's own path segments \
                 (e.g. NSE/INDEX/NIFTY or NSE/FNO/NIFTY/2019-03-28-1150000-CE)",
                crate::stored::clipped(instrument)
            ));
        }
    };
    if [exchange, segment, symbol]
        .into_iter()
        .chain(contract)
        .any(str::is_empty)
    {
        return Err(format!(
            "`{}` has an empty segment",
            crate::stored::clipped(instrument)
        ));
    }
    let contract = match contract {
        None => None,
        Some(word) => Some(Contract::parse(word).ok_or_else(|| {
            format!(
                "`{}` is not a contract segment the store renders (e.g. 2019-03-28-1150000-CE)",
                crate::stored::clipped(word)
            )
        })?),
    };
    let (year, number) = crate::srt_builder::month("YYYY-MM", month)?;
    let ask = Ask {
        vendor: vendor(feed)?,
        exchange: exchange.to_owned(),
        segment: segment.to_owned(),
        symbol: symbol.to_owned(),
        contract,
        rung: crate::stored::rung(rung)?,
        month: YearMonth::new(year, number).map_err(|why| format!("`{month}`: {why}"))?,
        write,
    };
    Ok(ask)
}

/// What the journal says of the dropped days, and what was written to it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Reread {
    /// Days the journal showed done, each given a read-again line.
    lines: Vec<String>,
    /// Days it did not show done: any Resume reads them already.
    open: Vec<String>,
    /// Why no journal answers for this month (another feed, a segment no
    /// import kind files), or `None` for a GDFL month.
    no_journal: Option<String>,
}

/// The import kind that files `ask`'s segment, for a GDFL month.
fn kind_of(ask: &Ask) -> Result<ImportKind, String> {
    if ask.vendor != Vendor::Gdfl {
        return Err(format!(
            "{} keeps no import journal; re-pull the dropped days through its own door",
            ask.vendor.as_str()
        ));
    }
    ImportKind::ALL
        .into_iter()
        .find(|kind| kind.segment() == ask.segment)
        .ok_or_else(|| {
            format!(
                "no GDFL import kind files the segment {}; read the dropped days again by hand",
                crate::stored::clipped(&ask.segment)
            )
        })
}

/// Which dropped days the journal shows done, read only.
fn journal_view(root: &Path, ask: &Ask, dropped: &Dropped) -> Result<Reread, String> {
    let kind = match kind_of(ask) {
        Ok(kind) => kind,
        Err(why) => {
            return Ok(Reread {
                open: dropped.days().map(|day| ask.date(day)).collect(),
                no_journal: Some(why),
                ..Reread::default()
            });
        }
    };
    // THE READER THAT WRITES NOTHING (`JournalReader`, IA-1): a torn last
    // line is kept back unread, never closed, so a plan stays a plan.
    let journal = pull::gdfl_run::JournalReader::open(root).map_err(|why| why.to_string())?;
    let mut view = Reread::default();
    for day in dropped.days() {
        let date = ask.date(day);
        let on = pull::session::Day::new(ask.month.year(), ask.month.month(), day);
        let on = on.map_err(|why| format!("{date}: {why}"))?;
        if journal.is_done(kind, on, "*") {
            view.lines.push(date);
        } else {
            view.open.push(date);
        }
    }
    Ok(view)
}

/// One read-again line, in the grammar the importer's own `incomplete` line
/// is written in (`pull::gdfl_import`'s journal, D-3317).
fn reread_line(kind: ImportKind, date: &str) -> String {
    format!(
        "incomplete {} {date} * definition={} files=0 seconds=0 failures=0 late=0 \
         late_unresolved=0 max_back_s=0 skipped=0 stage=file reason={REREAD_REASON}",
        kind.as_str(),
        pull::gdfl_import::BAR_DEFINITION
    )
}

/// Writes the read-again lines for the dropped days the journal shows done,
/// under the journal's own lock (waited for at most `wait`), never beside a
/// live GDFL run.
fn write_reread(
    root: &Path,
    ask: &Ask,
    dropped: &Dropped,
    wait: std::time::Duration,
) -> Result<Reread, String> {
    let view = journal_view(root, ask, dropped)?;
    if view.lines.is_empty() {
        return Ok(view);
    }
    if let Some(why) = crate::upkeep_command::live_run(root, Vendor::Gdfl) {
        return Err(format!(
            "{why}; the read-again lines wait for it (the recovery stands, and the same command \
             writes them once the run is over)"
        ));
    }
    let kind = kind_of(ask)?;
    let journal = pull::gdfl_import::journal_path(root);
    let lock = pull::gdfl_import::journal_lock_path(&journal);
    let held = pull::gdfl_run::locks::take_path_within(&lock, wait);
    let _held = held.map_err(|fault| format!("{}: {fault}", lock.display()))?;
    append_lines(
        &journal,
        &view
            .lines
            .iter()
            .map(|date| reread_line(kind, date))
            .collect::<Vec<_>>(),
    )?;
    Ok(view)
}

/// Appends whole lines to the journal and syncs it. A journal whose last
/// line is torn is refused, never joined: the importer closes its own torn
/// line on its next run.
fn append_lines(journal: &Path, lines: &[String]) -> Result<(), String> {
    use std::io::Write as _;
    use std::os::unix::fs::FileExt as _;
    let named = |why: std::io::Error| format!("{}: {why}", journal.display());
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .append(true)
        .open(journal)
        .map_err(named)?;
    // ITS LAST BYTE, or none: an empty journal reads nothing and ends whole.
    let len = file.metadata().map_err(named)?.len();
    let mut last = *b"\n";
    file.read_at(&mut last, len.saturating_sub(1))
        .map_err(named)?;
    if last != *b"\n" {
        return Err(format!(
            "{} ends in a torn line; the importer closes it on its next run, and the \
             read-again lines are not joined to it",
            journal.display()
        ));
    }
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    file.write_all(text.as_bytes()).map_err(named)?;
    file.sync_all().map_err(named)
}

/// The run's end record in `audit/<vendor>.recoveries`.
fn ledger_line(path: StorePath<'_>, ask: &Ask, record: &Record, reread: &Reread) -> String {
    let named = if reread.lines.is_empty() {
        "none".to_owned()
    } else {
        reread.lines.join(",")
    };
    format!(
        "recovered month={} revision={} basis={} generation={} kept={} claimed={} dropped={} \
         days={}..{} reread={named} recovered_at={} commit={}",
        path.to_path_buf(Path::new("")).display(),
        record.revision.ordinal(),
        record.basis.as_str(),
        record.basis.generation(),
        record.dropped.kept,
        record.dropped.claimed,
        record.dropped.bars,
        ask.date(record.dropped.first_day),
        ask.date(record.dropped.last_day),
        record.recovered_at_micros,
        crate::commit_stamp().unwrap_or("unstamped")
    )
}

/// What the journal says of the dropped days, as a receipt line.
fn render_view(out: &mut String, view: &Reread) {
    if let Some(why) = &view.no_journal {
        let _ = writeln!(out, "  re-read   {why}: {}", view.open.join(", "));
        return;
    }
    let list = |days: &[String]| {
        if days.is_empty() {
            "none".to_owned()
        } else {
            days.join(", ")
        }
    };
    let _ = writeln!(
        out,
        "  re-read   done in the import journal, given a read-again line: {}; not done, read by \
         any Resume: {}",
        list(&view.lines),
        list(&view.open)
    );
}

/// The refusal, said by name and logged as an error.
fn refused(out: &mut String, ask: &Ask, why: &RecoverError) -> u8 {
    let _ = writeln!(out, "MONTH RECOVER REFUSED: {why}");
    crate::note(
        &telemetry::Event::error(TARGET, "month recovery refused")
            .with("month", ask.named().as_str())
            .with("word", why.word())
            .with("why", why.to_string().as_str()),
    );
    FAILED
}

/// The plan, writing nothing.
fn run_plan(out: &mut String, root: &Path, ask: &Ask, path: StorePath<'_>) -> u8 {
    let plan = match store::recover::plan(root, path, ask.symbol_id()) {
        Ok(plan) => plan,
        Err(why) => return refused(out, ask, &why),
    };
    match plan {
        Plan::Clean { ordinal, n_valid } => {
            let _ = writeln!(
                out,
                "  clean     generation {ordinal} opens and every one of its {n_valid} bars \
                 verifies; nothing to recover"
            );
        }
        Plan::Recovered(record) => render_record(out, ask, &record),
        Plan::Cut {
            refused,
            cut,
            dropped,
            tried,
            promoted,
        } => {
            let _ = writeln!(out, "  refused   {refused}");
            for one in &tried {
                let _ = writeln!(
                    out,
                    "  tried     generation {} ({} bars): {}",
                    one.generation, one.n_valid, one.refused
                );
            }
            let header = cut.header();
            let _ = writeln!(
                out,
                "  cut       {}: keeps {} bars, {}..{}\n  claimed   generation {} published {} \
                 bars through {}\n  drops     {} bars on IST days {}..{} ({} day(s); the first \
                 may keep part of itself)",
                cut.basis(),
                dropped.kept,
                header.first_ts_micros,
                header.last_ts_micros,
                dropped.claimed_generation,
                dropped.claimed,
                dropped.claimed_last_ts,
                dropped.bars,
                ask.date(dropped.first_day),
                ask.date(dropped.last_day),
                dropped.days().count()
            );
            if let Some(pointer) = promoted {
                let _ = writeln!(
                    out,
                    "  promoted  revision {} is served already (a run cut off before its \
                     record); only the record is owed",
                    pointer.revision.ordinal()
                );
            }
            match journal_view(root, ask, &dropped) {
                Ok(view) => render_view(out, &view),
                Err(why) => {
                    let _ = writeln!(out, "  re-read   the import journal cannot be read: {why}");
                }
            }
            let _ = writeln!(
                out,
                "Nothing was written. `--yes` writes the kept bars as a new revision (the \
                 original is never changed) and names the dropped days for the re-read."
            );
        }
    }
    OK
}

fn render_record(out: &mut String, ask: &Ask, record: &Record) {
    let _ = writeln!(
        out,
        "  recovered revision {} is served: {}, {} bars kept, {} dropped on IST days {}..{}; the \
         re-read {}",
        record.revision.ordinal(),
        record.basis,
        record.dropped.kept,
        record.dropped.bars,
        ask.date(record.dropped.first_day),
        ask.date(record.dropped.last_day),
        if record.reread_named {
            "is named"
        } else {
            "is NOT named yet (a run cut off before it): `--yes` writes it"
        }
    );
}

/// The recovery, its re-read, its end record and its note.
fn run_yes(out: &mut String, root: &Path, ask: &Ask, path: StorePath<'_>) -> u8 {
    let (record, resumed) = match store::recover::recover(root, path, ask.symbol_id()) {
        Ok(Outcome::Clean { ordinal, n_valid }) => {
            let _ = writeln!(
                out,
                "  clean     generation {ordinal} opens and every one of its {n_valid} bars \
                 verifies; nothing to recover and nothing was written"
            );
            return OK;
        }
        Ok(Outcome::Already(record)) if record.reread_named => {
            render_record(out, ask, &record);
            let _ = writeln!(out, "Already recovered; nothing was written.");
            return OK;
        }
        Ok(Outcome::Already(record)) => (record, true),
        Ok(Outcome::Recovered(done)) => {
            let _ = writeln!(
                out,
                "  wrote     revision {} ({}; {} ordinal(s) stepped over), promoted; record {} at \
                 {}",
                done.record.revision.ordinal(),
                done.how(),
                done.skipped.len(),
                done.record.record,
                store::recover::record_path(root, path).display()
            );
            (done.record, done.resumed)
        }
        Err(why) => return refused(out, ask, &why),
    };
    render_record(out, ask, &record);
    let reread = match write_reread(root, ask, &record.dropped, pull::gdfl_run::locks::LOCK_WAIT) {
        Ok(reread) => reread,
        Err(why) => {
            let _ = writeln!(out, "MONTH RECOVER: the re-read was not named: {why}");
            crate::note(
                &telemetry::Event::error(TARGET, "month recovery re-read not named")
                    .with("month", ask.named().as_str())
                    .with("why", why.as_str()),
            );
            return FAILED;
        }
    };
    render_view(out, &reread);
    let line = ledger_line(path, ask, &record, &reread);
    let ended = end_record(root, ask.vendor, &line).and_then(|()| noted(root, path, &record));
    match ended {
        Ok(noted) => {
            crate::note(
                &telemetry::Event::info(TARGET, "month recovered")
                    .with("month", ask.named().as_str())
                    .with("revision", u64::from(noted.revision.ordinal()))
                    .with("basis", noted.basis.as_str())
                    .with("kept", noted.dropped.kept)
                    .with("dropped", noted.dropped.bars)
                    .with("first_day", ask.date(noted.dropped.first_day).as_str())
                    .with("last_day", ask.date(noted.dropped.last_day).as_str())
                    .with("reread_lines", reread.lines.len() as u64)
                    .with("resumed", resumed),
            );
            let _ = writeln!(
                out,
                "  ledger    audit/{}.{LEDGER}: {line}\nRecovered; the original month's bytes are \
                 unchanged.",
                ask.vendor.as_str()
            );
            OK
        }
        Err(why) => {
            let _ = writeln!(out, "MONTH RECOVER: {why}; the same command writes it");
            crate::note(
                &telemetry::Event::error(TARGET, "month recovery end record not written")
                    .with("month", ask.named().as_str())
                    .with("why", why.as_str()),
            );
            FAILED
        }
    }
}

/// The run's end record, one line of `audit/<vendor>.recoveries`.
fn end_record(root: &Path, vendor: Vendor, line: &str) -> Result<(), String> {
    let lines = [line.to_owned()];
    let appended = pull::audit_ledger::append(root, vendor, LEDGER, &lines);
    appended.map_err(|why| format!("the end record was not written: {why}"))
}

/// The recovery's record, marked: its re-read is named.
fn noted(root: &Path, path: StorePath<'_>, record: &Record) -> Result<Record, String> {
    let noted = store::recover::note_reread(root, path, record);
    noted.map_err(|why| format!("the re-read was not noted: {why}"))
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
    // THE PATH IS PROVEN BEFORE THE ROOT IS ASKED FOR, so a name the store
    // cannot hold is a misuse, never a refusal of the store.
    let path = match ask.path() {
        Ok(path) => path,
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
                "MONTH RECOVER REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    let _ = writeln!(
        out,
        "MONTH RECOVER{} — {} under {}",
        if ask.write {
            " (the operator's yes: writes the recovered month)"
        } else {
            " (plan: nothing is written)"
        },
        ask.named(),
        root.display()
    );
    if ask.write {
        run_yes(out, &root, &ask, path)
    } else {
        run_plan(out, &root, &ask, path)
    }
}

/// `cli month-recover`, against the CLI's store root (`BRUTEX_STORE`).
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // AN UNSTAMPED BUILD RECOVERS NOTHING (DB-B71, WP-81); `--plan` writes
    // nothing and is not refused.
    let store = crate::store_write::root_for("month-recover", arguments);
    command_with(arguments, &store, out)
}

#[cfg(test)]
#[path = "month_recover_command_tests.rs"]
mod tests;
