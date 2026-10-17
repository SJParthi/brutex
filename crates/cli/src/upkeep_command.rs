//! `cli upkeep` — the store's catch-up steps as tracked runs: what each stored
//! month lacks, and each builder run over it on N workers with a run id,
//! progress, Stop and resume, under the import's own locks. R3-17, D-4997;
//! DB-027, DB-N14.
//!
//! # Usage
//!
//! ```text
//! cli upkeep plan VENDOR [SCOPE] [--rung R] [--from YYYY-MM] [--to YYYY-MM]
//! cli upkeep run KIND VENDOR [SCOPE] [FLAGS]
//! cli upkeep resume KIND VENDOR RUN [SCOPE] [FLAGS]
//! cli upkeep stop KIND VENDOR [RUN]
//! cli upkeep status KIND VENDOR
//! ```
//!
//! `KIND` is `tix` (the day index), `srt` (the sort index), `msum` (the month
//! summary), `recount` (the census rows of each month's days) or `rekey` (the
//! GDFL option keys of each underlying). `SCOPE` is the store's own path
//! segments under `bars/<vendor>/`, as deep as wanted: `NSE`, `NSE/FNO`,
//! `NSE/INDEX/NIFTY`, `NSE/FNO/NIFTY/2019-03-28-1150000-CE`; absent, the
//! whole feed. A symbol holds its spot rungs and every contract beneath it.
//!
//! The flags of the three indexes: `--rung R`, `--from YYYY-MM`, `--to
//! YYYY-MM`, `--workers N` (1 to `pull::upkeep::MAX_WORKERS`; 1 when absent)
//! and `--all` (every month in scope, not only the months that lack the kind's
//! file: a month already done is kept after its checks, so `--all` re-checks
//! the feed). The recount takes `--from DAY --to DAY`, both required, and
//! recounts one calendar month of those days per item. The re-key takes
//! `--underlying SYMBOL[,SYMBOL...]` and is GDFL's alone.
//!
//! # What it does
//!
//! `plan` writes nothing: it walks the scope once (`pull::upkeep::months`)
//! and says, per series, how many months lack each index, and how the last
//! run of each kind ended. A month under 1,024 bars needs none of the three
//! (D-4933); the plan counts it as lacking and the run names it `small`.
//!
//! `run` is one tracked run (`pull::upkeep::run`): its journal is
//! `audit/upkeep.<vendor>.<kind>.log`, each item one line carrying the run's
//! totals, and each item is built by the verb that owns it — the very step
//! `cli tix-rebuild`, `srt-build`, `msum-build`, `census-recount` and
//! `gdfl-rekey` take, with the receipt line they print as its reason. The
//! store's month lock refuses a month an import holds; the run names it, and a
//! resume builds it once it is free.
//!
//! `resume` is `run` naming an earlier run: every item a run of its chain
//! closed clean is skipped. `stop` writes the request the running run reads
//! between items (`pull::upkeep::request_stop`); without `RUN` it names the
//! newest run, when one is running. `status` reads the journal's last line.
//!
//! # Locks: the import's own
//!
//! A GDFL run first takes the import's run lock of every kind
//! (`imports/gdfl.<kind>.lock` on the version-4 census, `imports/gdfl.lock` on
//! version 3), without waiting, the holder record naming this verb: while an
//! import of any kind runs, an upkeep run is refused `run_busy` by name, and
//! while an upkeep run holds them, an import is. Every kind, because a
//! recount writes every segment's census and the import journal, and a
//! capital-market import files more than one segment. Under them each month
//! is opened under its own exclusive store lock. Zerodha has no import run
//! lock: its runs meet a pull on the month locks alone.
//!
//! It never starts by itself: each kind is run on the operator's yes, from
//! the store-step list (`config/store-steps.toml`).
//!
//! # Exit
//!
//! [`OK`] for `run` and `resume` only when every item given was built clean
//! (a Stop, a refused month, a failed journal is [`FAILED`]); for `plan` and
//! `status` when the store could be read; for `stop` when a request was
//! written for a running run. [`MISUSED`] for arguments it does not know,
//! before anything is opened.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::gdfl_import::ImportKind;
use pull::ingest::census_store::Span;
use pull::session::Day;
use pull::upkeep::{self, Ask, Kind, Month, Outcome, Ran, Scope, State, Status};
use store::path::YearMonth;
use telemetry::run_dirs::GdflLayout;

use crate::gdfl_import_command::{Finding, Holder, RunLock, escape};
use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli upkeep plan VENDOR [SCOPE] [--rung R] [--from YYYY-MM] \
     [--to YYYY-MM] | cli upkeep run KIND VENDOR [SCOPE] [--rung R] [--from YYYY-MM] \
     [--to YYYY-MM] [--workers N] [--all] | cli upkeep run recount VENDOR --from DAY --to DAY | \
     cli upkeep run rekey gdfl [--underlying SYMBOL[,SYMBOL...]] | cli upkeep resume KIND VENDOR \
     RUN [the run's arguments] | cli upkeep stop KIND VENDOR [RUN] | cli upkeep status KIND \
     VENDOR  (KIND is tix, srt, msum, recount or rekey)";

/// Lines of a list a receipt prints before it only counts the rest.
const SHOWN: usize = 200;

/// What a `run` or `resume` names beside its kind and vendor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Flags {
    /// The scope of an index run.
    scope: Scope,
    /// Workers asked for.
    workers: usize,
    /// Every month in scope, not only those lacking the kind's file.
    all: bool,
    /// The days of a recount.
    days: Option<(Day, Day)>,
    /// The underlyings of a re-key.
    underlyings: Option<BTreeSet<Symbol>>,
}

/// What the arguments ask.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verb {
    Plan {
        vendor: Vendor,
        scope: Scope,
    },
    Run {
        kind: Kind,
        vendor: Vendor,
        resume: Option<u64>,
        flags: Flags,
    },
    Stop {
        kind: Kind,
        vendor: Vendor,
        id: Option<u64>,
    },
    Status {
        kind: Kind,
        vendor: Vendor,
    },
}

fn kind(word: &str) -> Result<Kind, String> {
    Kind::parse(word).ok_or_else(|| {
        format!(
            "`{}` is not an upkeep kind; the kinds are tix, srt, msum, recount and rekey",
            crate::stored::clipped(word)
        )
    })
}

fn run_id(word: &str) -> Result<u64, String> {
    word.parse()
        .map_err(|_| format!("`{}` is not a run id", crate::stored::clipped(word)))
}

fn year_month(label: &str, word: &str) -> Result<YearMonth, String> {
    let (year, number) = crate::srt_builder::month(label, word)?;
    YearMonth::new(year, number).map_err(|why| format!("{label} `{word}`: {why}"))
}

/// The scope's path segments: one to four, none empty, none a step out.
fn prefix(word: &str) -> Result<Vec<String>, String> {
    let parts: Vec<String> = word.split('/').map(str::to_owned).collect();
    if parts.len() > 4
        || parts
            .iter()
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!(
            "`{}` is not EXCHANGE[/SEGMENT[/SYMBOL[/CONTRACT]]], the store's own path segments",
            crate::stored::clipped(word)
        ));
    }
    Ok(parts)
}

/// The flags after the kind and vendor (and run id), refused by name. Each
/// is given at most once and only where its kind takes it.
fn flags(kind: Option<Kind>, vendor: Vendor, rest: &[&str]) -> Result<Flags, String> {
    let mut out = Flags {
        workers: 1,
        ..Flags::default()
    };
    let (mut seen, mut from_day, mut to_day) = (BTreeSet::new(), None, None);
    let per_month = kind.is_none_or(Kind::per_month);
    let mut words = rest.iter();
    while let Some(&word) = words.next() {
        if !word.starts_with("--") {
            if !per_month || !out.scope.prefix.is_empty() {
                return Err(format!(
                    "`{}` is not an argument here",
                    crate::stored::clipped(word)
                ));
            }
            out.scope.prefix = prefix(word)?;
            continue;
        }
        if !seen.insert(word) {
            return Err(format!("{word} is given twice"));
        }
        let takes = match word {
            "--rung" => per_month,
            "--from" | "--to" => per_month || kind == Some(Kind::Recount),
            "--workers" | "--all" => per_month && kind.is_some(),
            "--underlying" => kind == Some(Kind::Rekey),
            _ => false,
        };
        if !takes {
            return Err(format!(
                "`{}` is not a flag of upkeep {}",
                crate::stored::clipped(word),
                kind.map_or("plan", Kind::word)
            ));
        }
        if word == "--all" {
            out.all = true;
            continue;
        }
        let Some(&value) = words.next() else {
            return Err(format!("{word} needs a value"));
        };
        match (word, per_month) {
            ("--rung", _) => out.scope.rung = Some(crate::stored::rung(value)?),
            ("--from", true) => out.scope.from = Some(year_month("--from", value)?),
            ("--to", true) => out.scope.to = Some(year_month("--to", value)?),
            ("--from", false) => from_day = Some(crate::census_recount::day(value)?),
            ("--to", false) => to_day = Some(crate::census_recount::day(value)?),
            ("--workers", _) => {
                out.workers = value
                    .parse()
                    .ok()
                    .filter(|n| (1..=upkeep::MAX_WORKERS).contains(n))
                    .ok_or_else(|| {
                        format!(
                            "--workers `{}` is not 1 to {}",
                            crate::stored::clipped(value),
                            upkeep::MAX_WORKERS
                        )
                    })?;
            }
            _ => out.underlyings = Some(underlyings(value)?),
        }
    }
    if let (Some(from), Some(to)) = (out.scope.from, out.scope.to)
        && to < from
    {
        return Err(format!("--to {to} is before --from {from}"));
    }
    match (kind, from_day, to_day) {
        (Some(Kind::Recount), Some(from), Some(to)) if to >= from => out.days = Some((from, to)),
        (Some(Kind::Recount), Some(from), Some(to)) => {
            return Err(format!("--to {to} is before --from {from}"));
        }
        (Some(Kind::Recount), _, _) => {
            return Err(
                "upkeep recount needs --from DAY and --to DAY: the span decides which import \
                 days a recount closes, so it is never implied (D-3317)"
                    .to_owned(),
            );
        }
        _ => {}
    }
    if kind == Some(Kind::Rekey) && vendor != Vendor::Gdfl {
        return Err("the re-key is GDFL's alone (D-3331)".to_owned());
    }
    Ok(out)
}

/// `SYMBOL[,SYMBOL...]`, each a store symbol.
fn underlyings(list: &str) -> Result<BTreeSet<Symbol>, String> {
    list.split(',')
        .map(|word| {
            Symbol::new(word)
                .ok()
                .filter(|symbol| symbol.as_str() == word)
                .ok_or_else(|| format!("`{}` is not a store symbol", crate::stored::clipped(word)))
        })
        .collect()
}

/// The arguments, refused by name before anything is opened.
fn verb(arguments: &[&str]) -> Result<Verb, String> {
    match arguments {
        ["plan", feed, rest @ ..] => {
            let vendor = crate::srt_builder::vendor(feed)?;
            Ok(Verb::Plan {
                vendor,
                scope: flags(None, vendor, rest)?.scope,
            })
        }
        ["run", word, feed, rest @ ..] => {
            let (kind, vendor) = (kind(word)?, crate::srt_builder::vendor(feed)?);
            Ok(Verb::Run {
                kind,
                vendor,
                resume: None,
                flags: flags(Some(kind), vendor, rest)?,
            })
        }
        ["resume", word, feed, id, rest @ ..] => {
            let (kind, vendor) = (kind(word)?, crate::srt_builder::vendor(feed)?);
            Ok(Verb::Run {
                kind,
                vendor,
                resume: Some(run_id(id)?),
                flags: flags(Some(kind), vendor, rest)?,
            })
        }
        ["stop", word, feed, id @ ..] if id.len() <= 1 => Ok(Verb::Stop {
            kind: kind(word)?,
            vendor: crate::srt_builder::vendor(feed)?,
            id: id.first().map(|id| run_id(id)).transpose()?,
        }),
        ["status", word, feed] => Ok(Verb::Status {
            kind: kind(word)?,
            vendor: crate::srt_builder::vendor(feed)?,
        }),
        _ => Err("upkeep takes plan, run, resume, stop or status, with its arguments".to_owned()),
    }
}

/// A builder's outcome from the receipt line its verb prints for the item,
/// `label` first: the word is what follows the label, the reason the line.
pub(crate) fn outcome(clean: bool, label: &str, receipt: &str) -> Outcome {
    let line = receipt.trim();
    let rest = line.strip_prefix(label).unwrap_or(line).trim_start();
    let word = rest
        .split("  ")
        .next()
        .unwrap_or("-")
        .trim()
        .replace(' ', "_");
    Outcome {
        clean,
        word,
        why: line.to_owned(),
    }
}

/// One item of a run: what its builder is handed.
enum Item {
    /// A month file of the scope.
    Month(Month),
    /// One calendar month of a recount's days.
    Span(Span),
    /// One underlying of a re-key.
    Underlying(Symbol),
}

/// A recount span's key: `<from>..<to>`.
fn span_key(span: Span) -> String {
    format!(
        "{}..{}",
        span.from,
        span.to.map_or_else(String::new, |to| to.to_string())
    )
}

/// The day spans of `from..=to`, one per calendar month.
fn spans(from: Day, to: Day) -> Vec<Span> {
    let mut out = Vec::new();
    let mut start = from;
    loop {
        let end = start.end_of_month().min(to);
        out.push(Span {
            from: start,
            to: Some(end),
        });
        match end.succ() {
            Ok(next) if next <= to => start = next,
            _ => return out,
        }
    }
}

/// The underlyings under `bars/gdfl/NSE/FNO/`, in name order, narrowed to
/// `only`; an underlying `only` names that holds no directory is refused.
fn underlyings_held(root: &Path, only: Option<&BTreeSet<Symbol>>) -> Result<Vec<Symbol>, String> {
    let dir = root.join("bars/gdfl/NSE/FNO");
    let mut held = BTreeSet::new();
    let listed =
        std::fs::read_dir(&dir).map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
    for entry in listed {
        let entry = entry.map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if let (true, Ok(symbol)) = (is_dir, Symbol::new(&name))
            && symbol.as_str() == name
        {
            let _ = held.insert(symbol);
        }
    }
    match only {
        None => Ok(held.into_iter().collect()),
        Some(only) => {
            for symbol in only {
                if !held.contains(symbol) {
                    return Err(format!(
                        "{} holds no option directory for {}",
                        dir.display(),
                        symbol.as_str()
                    ));
                }
            }
            Ok(only.iter().copied().collect())
        }
    }
}

/// The items `kind` builds under `root`, each with its key, and what they
/// are in words.
fn items(
    root: &Path,
    kind: Kind,
    vendor: Vendor,
    flags: &Flags,
) -> Result<(Vec<(String, Item)>, String), String> {
    match (kind, flags.days) {
        (Kind::Recount, Some((from, to))) => Ok((
            spans(from, to)
                .into_iter()
                .map(|span| (span_key(span), Item::Span(span)))
                .collect(),
            format!("the days {from} .. {to}, one calendar month an item"),
        )),
        (Kind::Rekey, _) => Ok((
            underlyings_held(root, flags.underlyings.as_ref())?
                .into_iter()
                .map(|symbol| (symbol.as_str().to_owned(), Item::Underlying(symbol)))
                .collect(),
            "the option underlyings under bars/gdfl/NSE/FNO, one an item".to_owned(),
        )),
        _ => {
            let walked = upkeep::months(root, vendor, &flags.scope)?;
            let total = walked.months.len();
            let months: Vec<(String, Item)> = walked
                .months
                .into_iter()
                .filter(|month| flags.all || month.lacks.has(kind))
                .map(|month| (month.key.clone(), Item::Month(month)))
                .collect();
            let said = format!(
                "{} of the {total} month file(s) in scope{}; {} link(s) never followed, {} stray \
                 name(s) left alone",
                months.len(),
                if flags.all {
                    " (--all: every one)"
                } else {
                    " that have no file of this kind"
                },
                walked.links,
                walked.strays
            );
            Ok((months, said))
        }
    }
}

/// Builds one item, by the step of the verb that owns it.
fn build_item(
    root: &Path,
    kind: Kind,
    vendor: Vendor,
    item: &Item,
    commit: Option<&str>,
) -> Outcome {
    match item {
        Item::Month(month) => match month.path(vendor) {
            Ok(path) => {
                let id = crate::srt_builder::symbol_id(&month.symbol);
                match kind {
                    Kind::DayIndex => crate::tix_rebuilder::upkeep_month(root, path, id),
                    Kind::SortIndex => crate::srt_builder::upkeep_month(root, path, id),
                    _ => crate::msum_builder::upkeep_month(root, path, id),
                }
            }
            // UNREACHABLE: the walk proved the month's path when it found it
            // (`pull::upkeep::months`); said, never assumed.
            Err(why) => Outcome::failed("REFUSED", why),
        },
        Item::Span(span) => crate::census_recount::upkeep_span(root, vendor, *span, commit),
        Item::Underlying(symbol) => crate::gdfl_rekey::upkeep_underlying(root, *symbol),
    }
}

/// The import's run locks this run takes, every kind's, without waiting. On
/// a version-3 census one lock serialises every kind, so it is taken once.
pub(crate) fn run_locks(
    root: &Path,
    vendor: Vendor,
    verb: &str,
    started: u64,
) -> Result<Vec<RunLock>, String> {
    let mut held = Vec::new();
    if vendor != Vendor::Gdfl {
        return Ok(held);
    }
    for kind in ImportKind::ALL {
        let holder = Holder {
            pid: std::process::id(),
            run_started: started,
            kind,
            verb: escape(verb.as_bytes()),
        };
        let lock = RunLock::take(root, &holder).map_err(sentence)?;
        let single = lock.layout == GdflLayout::V3;
        held.push(lock);
        if single {
            break;
        }
    }
    Ok(held)
}

/// The import door's finding as one sentence: its code, then its words.
fn sentence(found: Finding) -> String {
    let Finding { code, words } = found;
    format!("{code}: {words}")
}

/// UNIX seconds now: the binary's clock.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// What the binary tells once an item is built: nothing, beside the journal
/// line the run writes for it.
const fn no_progress(_: &str, _: &Outcome) {}

/// What releasing the locks came to: every refusal named, or none.
fn all_released(results: impl IntoIterator<Item = Result<(), String>>) -> Result<(), String> {
    let refused: Vec<String> = results.into_iter().filter_map(Result::err).collect();
    if refused.is_empty() {
        Ok(())
    } else {
        Err(refused.join("; "))
    }
}

/// The rate of a run, in items a second to one decimal, with no float.
fn rate(items: usize, millis: u64) -> String {
    let items = u64::try_from(items).unwrap_or(u64::MAX);
    if millis == 0 {
        return format!("{items} item(s) in under 1 ms");
    }
    let tenths = items.saturating_mul(10_000) / millis;
    format!(
        "{}.{} item(s) a second ({items} in {millis} ms)",
        tenths / 10,
        tenths % 10
    )
}

/// The receipt of one run.
fn render_run(
    out: &mut String,
    (kind, vendor): (Kind, Vendor),
    said: &str,
    ran: &Ran,
    plan: &[(String, Item)],
) {
    let _ = writeln!(
        out,
        "  run {}{}: {} worker(s), {} item(s) given, {} skipped as closed clean by the run(s) \
         resumed",
        ran.id,
        ran.resumed
            .map_or_else(String::new, |resumed| format!(" (resumes run {resumed})")),
        ran.workers,
        plan.len() - ran.skipped,
        ran.skipped
    );
    let _ = writeln!(out, "  items               {said}");
    if ran.torn > 0 {
        let _ = writeln!(
            out,
            "  torn tail           {} byte(s) a killed run left were ended as a void line",
            ran.torn
        );
    }
    let mut words: BTreeMap<&str, usize> = BTreeMap::new();
    for outcome in ran.outcomes.iter().flatten() {
        *words.entry(outcome.word.as_str()).or_default() += 1;
    }
    let tally: Vec<String> = words
        .iter()
        .map(|(word, n)| format!("{word} {n}"))
        .collect();
    let _ = writeln!(
        out,
        "  outcomes            {}",
        if tally.is_empty() {
            "none".to_owned()
        } else {
            tally.join(", ")
        }
    );
    let mut shown = 0;
    for ((key, _), outcome) in plan.iter().zip(&ran.outcomes) {
        if let Some(outcome) = outcome.as_ref().filter(|outcome| !outcome.clean) {
            if shown < SHOWN {
                let _ = writeln!(out, "    NOT CLEAN {key}: {}", outcome.why);
            }
            shown += 1;
        }
    }
    if shown > SHOWN {
        let _ = writeln!(out, "    ... and {} more not clean", shown - SHOWN);
    }
    let _ = writeln!(
        out,
        "  rate                {}",
        rate(ran.built(), ran.millis)
    );
    let _ = writeln!(
        out,
        "  ENDED {}: {} ({} built, {} clean, {} left)",
        ran.how.word(),
        ran.why,
        ran.built(),
        ran.clean(),
        ran.left()
    );
    if !ran.whole() {
        let _ = writeln!(
            out,
            "  resume with: cli upkeep resume {} {} {} [the same arguments]",
            kind.word(),
            vendor.as_str(),
            ran.id
        );
    }
}

/// One status line.
fn render_status(out: &mut String, kind: Kind, status: Option<&Status>) {
    let Some(status) = status else {
        let _ = writeln!(out, "  {:<8} no run yet", kind.word());
        return;
    };
    let state = match status.state {
        State::Running => "RUNNING".to_owned(),
        State::Ended(how) => format!("ended {}", how.word()),
        State::Died => "DIED (no end line, and its lock is free)".to_owned(),
    };
    let eta = status.eta().map_or_else(String::new, |eta| {
        format!(", about {eta} s left at its rate so far (an extrapolation)")
    });
    let _ = writeln!(
        out,
        "  {:<8} run {} {state}: {} of {} built, {} clean, {} s{eta}",
        kind.word(),
        status.id,
        status.n,
        status.of,
        status.ok,
        status.t
    );
}

/// The plan: what each series' months lack, and how each kind's last run
/// ended.
fn plan(out: &mut String, root: &Path, vendor: Vendor, scope: &Scope) -> u8 {
    let walked = match upkeep::months(root, vendor, scope) {
        Ok(walked) => walked,
        Err(why) => {
            let _ = writeln!(out, "UPKEEP PLAN REFUSED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    let _ = writeln!(
        out,
        "UPKEEP PLAN — {} {} under {}: {} month file(s) (nothing is written)",
        vendor.as_str(),
        if scope.prefix.is_empty() {
            "(the whole feed)".to_owned()
        } else {
            scope.prefix.join("/")
        },
        root.display(),
        walked.months.len()
    );
    let mut groups: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    for month in &walked.months {
        let series = format!(
            "{}/{}/{}{} {}",
            month.exchange,
            month.segment,
            month.symbol,
            if month.contract.is_some() { "/*" } else { "" },
            month.rung.as_str()
        );
        let row = groups.entry(series).or_default();
        row[0] += 1;
        let kinds = [Kind::DayIndex, Kind::SortIndex, Kind::MonthSummary];
        for (count, kind) in row.iter_mut().skip(1).zip(kinds) {
            *count += usize::from(month.lacks.has(kind));
        }
    }
    let mut totals = [0_usize; 4];
    for (at, (series, row)) in groups.iter().enumerate() {
        for (total, n) in totals.iter_mut().zip(row) {
            *total += n;
        }
        if at < SHOWN {
            let _ = writeln!(
                out,
                "  {series:<40} {:>7} month(s); lacks .tix {:>6}, .srt {:>6}, .sum {:>6}",
                row[0], row[1], row[2], row[3]
            );
        }
    }
    if groups.len() > SHOWN {
        let _ = writeln!(out, "  ... and {} more series", groups.len() - SHOWN);
    }
    let _ = writeln!(
        out,
        "  TOTAL lacks .tix {}, .srt {}, .sum {} of {} month(s); a month under 1,024 bars needs \
         none and a run says `small` for it (D-4933)",
        totals[1], totals[2], totals[3], totals[0]
    );
    let _ = writeln!(
        out,
        "  NOT WALKED: {} symbolic link(s), never followed, and {} name(s) that are not part of a \
         store path",
        walked.links, walked.strays
    );
    for path in walked.strays_named.iter().take(SHOWN) {
        let _ = writeln!(out, "    {}", path.display());
    }
    if vendor == Vendor::Gdfl {
        match underlyings_held(root, None) {
            Ok(symbols) => {
                let _ = writeln!(
                    out,
                    "  rekey: {} option underlying(s) under bars/gdfl/NSE/FNO, one item each \
                     (`cli gdfl-rekey --plan` names every key it would move or retire)",
                    symbols.len()
                );
            }
            Err(why) => {
                let _ = writeln!(out, "  rekey: {why}");
            }
        }
    }
    let _ = writeln!(
        out,
        "  recount: one item a calendar month of the days asked: cli upkeep run recount {} \
         --from DAY --to DAY",
        vendor.as_str()
    );
    let _ = writeln!(out, "  LAST RUNS");
    let mut clean = true;
    for kind in Kind::ALL {
        match upkeep::status(root, vendor, kind) {
            Ok(status) => render_status(out, kind, status.as_ref()),
            Err(why) => {
                clean = false;
                let _ = writeln!(out, "  {:<8} UNREADABLE: {why}", kind.word());
            }
        }
    }
    if clean { OK } else { FAILED }
}

/// What the binary hands a verb: its store root, its commit, its clock, the
/// in-process Stop (the app's, for a run it starts itself) and what to tell
/// once each item is built (its progress).
pub(crate) struct Host<'a> {
    pub(crate) store: &'a dyn Fn() -> Result<PathBuf, String>,
    pub(crate) commit: Option<&'a str>,
    pub(crate) now: &'a (dyn Fn() -> u64 + Sync),
    pub(crate) stop: &'a AtomicBool,
    pub(crate) after_item: &'a (dyn Fn(&str, &Outcome) + Sync),
}

/// `cli upkeep`, against what `host` names, so every arm is driven by a test
/// without the process environment.
pub(crate) fn command_with(arguments: &[&str], host: &Host<'_>, out: &mut String) -> u8 {
    let asked = match verb(arguments) {
        Ok(asked) => asked,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let root = match (host.store)() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(out, "UPKEEP REFUSED: {why}. Nothing was read or written.");
            return FAILED;
        }
    };
    match asked {
        Verb::Plan { vendor, scope } => plan(out, &root, vendor, &scope),
        Verb::Status { kind, vendor } => match upkeep::status(&root, vendor, kind) {
            Ok(status) => {
                let _ = writeln!(out, "UPKEEP STATUS — {} {}", vendor.as_str(), kind.word());
                render_status(out, kind, status.as_ref());
                OK
            }
            Err(why) => {
                let _ = writeln!(out, "UPKEEP STATUS REFUSED: {why}");
                FAILED
            }
        },
        Verb::Stop { kind, vendor, id } => stop(out, &root, kind, vendor, id),
        Verb::Run {
            kind,
            vendor,
            resume,
            flags,
        } => run(out, &root, host, (kind, vendor, resume), &flags, arguments),
    }
}

/// `stop`: the request, for the run named or the newest one running.
fn stop(out: &mut String, root: &Path, kind: Kind, vendor: Vendor, id: Option<u64>) -> u8 {
    let status = match upkeep::status(root, vendor, kind) {
        Ok(status) => status,
        Err(why) => {
            let _ = writeln!(out, "UPKEEP STOP REFUSED: {why}; nothing was asked");
            return FAILED;
        }
    };
    let running = status
        .as_ref()
        .filter(|status| status.state == State::Running)
        .map(|status| status.id);
    let Some(target) = id.or(running) else {
        let _ = writeln!(
            out,
            "UPKEEP STOP: no {} run of {} is running; nothing was asked",
            kind.word(),
            vendor.as_str()
        );
        return FAILED;
    };
    match upkeep::request_stop(root, vendor, kind, target) {
        Ok(path) if running == Some(target) => {
            let _ = writeln!(
                out,
                "UPKEEP STOP — asked run {target} to stop in {}: it finishes the item it holds \
                 and takes no other",
                path.display()
            );
            OK
        }
        Ok(path) => {
            let _ = writeln!(
                out,
                "UPKEEP STOP — wrote {} for run {target}, which is NOT running: it stops nothing",
                path.display()
            );
            FAILED
        }
        Err(why) => {
            let _ = writeln!(out, "UPKEEP STOP REFUSED: {why}; nothing was asked");
            FAILED
        }
    }
}

/// `run` and `resume`.
fn run(
    out: &mut String,
    root: &Path,
    host: &Host<'_>,
    (kind, vendor, resume): (Kind, Vendor, Option<u64>),
    flags: &Flags,
    arguments: &[&str],
) -> u8 {
    let heading = format!(
        "UPKEEP {} — {} for {}",
        if resume.is_some() { "RESUME" } else { "RUN" },
        kind.word(),
        vendor.as_str()
    );
    let (plan, said) = match items(root, kind, vendor, flags) {
        Ok(found) => found,
        Err(why) => {
            let _ = writeln!(out, "{heading} REFUSED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    let verb_line = format!("upkeep {}", arguments.join(" "));
    let held = match run_locks(root, vendor, &verb_line, (host.now)()) {
        Ok(held) => held,
        Err(why) => {
            let _ = writeln!(out, "{heading} REFUSED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    let _ = writeln!(out, "{heading} under {}", root.display());
    let ask = Ask {
        vendor,
        kind,
        workers: flags.workers,
        resume,
        commit: host.commit,
        stop: host.stop,
        now: host.now,
    };
    let (commit, after_item) = (host.commit, host.after_item);
    let build = |key: &str, item: &Item| {
        let outcome = build_item(root, kind, vendor, item, commit);
        after_item(key, &outcome);
        outcome
    };
    let ran = upkeep::run(root, &ask, &plan, &build);
    let released = all_released(
        held.into_iter()
            .map(|lock| lock.release().map_err(sentence)),
    );
    let ran = match ran {
        Ok(ran) => ran,
        Err(why) => {
            let _ = writeln!(out, "{heading} REFUSED: {why}");
            return FAILED;
        }
    };
    render_run(out, (kind, vendor), &said, &ran, &plan);
    exit_of(out, ran.whole(), released)
}

/// The exit of a run: [`OK`] only when it was whole and every lock it took
/// was released; a lock left held is named.
fn exit_of(out: &mut String, whole: bool, released: Result<(), String>) -> u8 {
    match released {
        Err(why) => {
            let _ = writeln!(out, "  LOCKS NOT RELEASED: {why}");
            FAILED
        }
        Ok(()) if whole => OK,
        Ok(()) => FAILED,
    }
}

/// `cli upkeep`, against the CLI's store root (`BRUTEX_STORE`), stamped with
/// the build's commit.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // AN UNSTAMPED BUILD RUNS NO UPKEEP (DB-B71, WP-81); `plan`, `status` and
    // `stop` write no store data and are not refused.
    let store = crate::store_write::root_for("upkeep", arguments);
    let stop = AtomicBool::new(false);
    let host = Host {
        store: &store,
        commit: crate::commit_stamp(),
        now: &unix_now,
        stop: &stop,
        after_item: &no_progress,
    };
    command_with(arguments, &host, out)
}

#[cfg(test)]
#[path = "upkeep_command_tests.rs"]
mod tests;
