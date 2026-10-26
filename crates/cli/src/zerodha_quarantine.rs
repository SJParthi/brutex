//! `cli zerodha-quarantine` — set aside the bars R4-17's quarantine plan
//! names, never in place. R4-18; findings ING-01, RT-DATA-02, Z5, Z6.
//!
//! # Usage
//!
//! ```text
//! cli zerodha-quarantine SYMBOL|ALL FROM TO --cause before_listing|foreign_level|zero_price
//!     --reason TEXT [--segment CASH|INDEX] [--plan-file PATH] [--plan|--apply]
//! ```
//!
//! `FROM` and `TO` are IST days, `YYYY-MM-DD`, inclusive. `--plan` (the
//! default) writes nothing; `--apply` writes.
//!
//! # What it reads
//!
//! The newest quarantine plan `cli series-identity` wrote for the feed and
//! segment (`<store>/audit/series-identity/zerodha-<SEGMENT>-<scope>-*.plan`,
//! scope the symbol or `ALL`), or the one `--plan-file` names. Its rows of
//! action `quarantine` and the cause asked, for the symbol, in the window,
//! are what is set aside: a `whole_month` row names every bar the month
//! holds, a `timestamps` row its listed stamps. For `before_listing` each
//! stamp is checked against the share's listing day once more
//! (`pull::listing`, O(1) per symbol): a stamp on or after the listing day is
//! refused by name and nothing of that month is written.
//!
//! # What `--apply` does
//!
//! Month by month, oldest first, through `store::quarantine::apply_month`:
//! a month whose every bar is named is RECORDED (no empty generation); any
//! other gets a new generation without the named bars through
//! `store::repair::drop_month`, the old one byte-identical. Each act is
//! ledgered in `<store>/audit/zerodha.quarantine.ledger` before the door
//! publishes, and indexed after. A minute month's derived rungs (2 to 60
//! minutes) are then set aside the same way: every derived bar whose span
//! holds a named minute, with the same cause — a bucket folded from a
//! foreign minute is never served. Idempotent: a second `--apply` writes
//! nothing, and a run killed mid-month is finished by the next.
//!
//! NOT DONE, said here: the derived bucket that mixes a named minute and a
//! kept one is set aside whole rather than folded again from the kept
//! minutes (the fold of a repair, `pull::ingest`'s derive path, refuses a
//! changed held bar and is not reachable from here). For `before_listing`
//! and `zero_price` every such bucket is wholly named; for `foreign_level`
//! (PIIND) a mixed bucket loses its kept minutes' share until a refold door
//! exists.
//!
//! # Exit
//!
//! [`OK`] when every month was set aside or already was; [`FAILED`] when a
//! month was refused (named); [`MISUSED`] for arguments that name nothing.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::session::{Day, IstMoment};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::{Applied, Cause, Why};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli zerodha-quarantine SYMBOL|ALL FROM TO --cause before_listing|foreign_level|zero_price --reason TEXT [--segment CASH|INDEX] [--plan-file PATH] [--plan|--apply]";

/// The first line every plan this verb reads carries (R4-17's `v2`).
pub(crate) const PLAN_HEAD: &str = "brutex series-identity quarantine plan v2";

/// What the arguments ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ask {
    /// `None` for `ALL`.
    pub(crate) symbol: Option<String>,
    pub(crate) from: Day,
    pub(crate) to: Day,
    pub(crate) cause: Cause,
    pub(crate) reason: String,
    pub(crate) segment: &'static str,
    pub(crate) plan_file: Option<PathBuf>,
    pub(crate) apply: bool,
}

fn parse_day(text: &str) -> Result<Day, String> {
    let mut parts = text.splitn(3, '-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(format!("`{text}` is not a day spelled YYYY-MM-DD"));
    };
    let number = |field: &str| {
        field
            .parse::<u16>()
            .map_err(|_| format!("`{text}` is not a day spelled YYYY-MM-DD"))
    };
    let (year, month, day) = (number(year)?, number(month)?, number(day)?);
    Day::new(
        year,
        u8::try_from(month).map_err(|_| format!("`{text}`: no such month"))?,
        u8::try_from(day).map_err(|_| format!("`{text}`: no such day"))?,
    )
    .map_err(|why| format!("`{text}`: {why}"))
}

/// Parses the verb's arguments.
///
/// # Errors
///
/// A sentence naming the argument refused.
pub(crate) fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let [first, from, to, flags @ ..] = arguments else {
        return Err("name a symbol or ALL, then FROM and TO days".to_owned());
    };
    if first.starts_with('-') {
        return Err(format!("`{first}` is a flag: name a symbol, or ALL, first"));
    }
    let symbol = if *first == "ALL" {
        None
    } else {
        // THE CORE'S CANONICAL SPELLING, the one the store and the plan key
        // a series by: `irfc` is `IRFC`, never a silent "nothing to do".
        let symbol =
            brutex_core::symbol::Symbol::new(first).map_err(|why| format!("`{first}`: {why}"))?;
        Some(symbol.as_str().to_owned())
    };
    let (from, to) = (parse_day(from)?, parse_day(to)?);
    if from > to {
        return Err(format!("FROM {from} is after TO {to}"));
    }
    let (mut cause, mut reason, mut segment, mut plan_file, mut mode) =
        (None, None, "CASH", None, None);
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        let mut value = |name: &str| {
            rest.next()
                .map(|value| (*value).to_owned())
                .ok_or_else(|| format!("{name} takes a value"))
        };
        match *flag {
            "--cause" => {
                let word = value("--cause")?;
                cause = Some(Cause::parse(&word).ok_or_else(|| {
                    format!("--cause `{word}` is none of before_listing, foreign_level, zero_price")
                })?);
            }
            "--reason" => reason = Some(value("--reason")?),
            "--segment" => {
                segment = match value("--segment")?.as_str() {
                    "CASH" => "CASH",
                    "INDEX" => "INDEX",
                    other => return Err(format!("--segment `{other}` is neither CASH nor INDEX")),
                };
            }
            "--plan-file" => plan_file = Some(PathBuf::from(value("--plan-file")?)),
            "--plan" | "--apply" => {
                let apply = *flag == "--apply";
                if mode.is_some_and(|was| was != apply) {
                    return Err("--plan and --apply are each other's opposite: name one".to_owned());
                }
                mode = Some(apply);
            }
            other => return Err(format!("`{other}` is not a flag of this verb")),
        }
    }
    let cause = cause.ok_or("name the cause with --cause")?;
    let reason = reason.ok_or("name the reason with --reason")?;
    store::quarantine::check_reason(&reason).map_err(|why| why.to_string())?;
    Ok(Ask {
        symbol,
        from,
        to,
        cause,
        reason,
        segment,
        plan_file,
        apply: mode.unwrap_or(false),
    })
}

/// One plan row this verb acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) symbol: String,
    pub(crate) rung: Timeframe,
    pub(crate) month: YearMonth,
    /// `None`: the whole month.
    pub(crate) stamps: Option<Vec<i64>>,
}

/// The plan's rows for `ask`, in key, rung, month order. A plan whose head is
/// not [`PLAN_HEAD`] is refused by name.
///
/// # Errors
///
/// A sentence naming the plan and the line.
pub(crate) fn rows_of(text: &str, ask: &Ask) -> Result<Vec<Row>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(PLAN_HEAD) {
        return Err(format!(
            "the plan does not begin `{PLAN_HEAD}`; it is not read"
        ));
    }
    let prefix = format!("NSE-{}-", ask.segment);
    let (first_month, last_month) = (month_of(ask.from)?, month_of(ask.to)?);
    let mut rows = Vec::new();
    let mut table = false;
    for line in lines {
        if line.starts_with("key\trung\t") {
            table = true;
            continue;
        }
        if !table {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [key, rung, month, scope, cause, action, bars, stamps] = fields.as_slice() else {
            return Err(format!(
                "a plan row has {} fields, not 8: `{line}`",
                fields.len()
            ));
        };
        let Some(symbol) = key.strip_prefix(prefix.as_str()) else {
            continue;
        };
        if ask.symbol.as_deref().is_some_and(|wanted| wanted != symbol)
            || *cause != ask.cause.code()
            || *action != "quarantine"
        {
            continue;
        }
        let rung = match *rung {
            "1min" => Timeframe::MINUTE_1,
            "1day" => Timeframe::DAY_1,
            other => return Err(format!("a plan row names rung `{other}`: `{line}`")),
        };
        let month =
            parse_month(month).ok_or_else(|| format!("a plan row names month `{month}`"))?;
        if month < first_month || month > last_month {
            continue;
        }
        let stamps = match *scope {
            "whole_month" => None,
            "timestamps" => {
                let listed = stamps
                    .split(',')
                    .map(str::parse::<i64>)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| format!("a plan row's stamps do not parse: `{key} {month}`"))?;
                if bars.parse::<usize>().ok() != Some(listed.len()) {
                    return Err(format!(
                        "a plan row counts {bars} bars and lists {}: `{key} {month}`",
                        listed.len()
                    ));
                }
                // A STAMP LISTED TWICE is a malformed row, named; it is never
                // silently folded into one.
                let mut sorted = listed.clone();
                sorted.sort_unstable();
                let twice = sorted.windows(2).find_map(|pair| match pair {
                    [one, two] if one == two => Some(*one),
                    _ => None,
                });
                if let Some(twice) = twice {
                    return Err(format!(
                        "a plan row lists stamp {twice} twice: `{key} {month}`"
                    ));
                }
                Some(listed)
            }
            other => return Err(format!("a plan row names scope `{other}`")),
        };
        rows.push(Row {
            symbol: symbol.to_owned(),
            rung,
            month,
            stamps,
        });
    }
    Ok(rows)
}

fn month_of(day: Day) -> Result<YearMonth, String> {
    YearMonth::new(day.year(), day.month()).map_err(|why| why.to_string())
}

fn parse_month(text: &str) -> Option<YearMonth> {
    let (year, month) = text.split_once('-')?;
    YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()
}

/// The plan to read: `--plan-file`, or the newest plan for the feed, the
/// segment and the symbol's scope (or `ALL`).
fn plan_path(root: &Path, ask: &Ask) -> Result<PathBuf, String> {
    if let Some(path) = &ask.plan_file {
        return Ok(path.clone());
    }
    let dir = root.join("audit").join("series-identity");
    let entries = std::fs::read_dir(&dir).map_err(|why| {
        format!(
            "no quarantine plan: {} would not list ({why}); run `cli series-identity {} --segment {}` first",
            dir.display(),
            ask.symbol.as_deref().unwrap_or("ALL"),
            ask.segment
        )
    })?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let scopes = [ask.symbol.as_deref().unwrap_or("ALL"), "ALL"];
        // THE NAME R4-17 WRITES, WHOLE: `zerodha-<SEG>-<SCOPE>-<16 hex>.plan`.
        // A prefix test alone read `X-Y`'s plan for `X` (R4-18 repair 2).
        let fits = scopes.iter().any(|scope| {
            name.strip_prefix(&format!("zerodha-{}-{scope}-", ask.segment))
                .and_then(|rest| rest.strip_suffix(".plan"))
                .is_some_and(|hex| hex.len() == 16 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        });
        if !fits {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if newest.as_ref().is_none_or(|(at, _)| modified > *at) {
            newest = Some((modified, entry.path()));
        }
    }
    newest.map(|(_, path)| path).ok_or_else(|| {
        format!(
            "no quarantine plan for zerodha {} {} under {}",
            ask.segment,
            ask.symbol.as_deref().unwrap_or("ALL"),
            dir.display()
        )
    })
}

fn path_of<'a>(
    segment: &'a str,
    symbol: &'a str,
    rung: Timeframe,
    month: YearMonth,
) -> Result<StorePath<'a>, String> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: rung,
        month,
        file: FileKind::Bars,
    })
    .map_err(|why| why.to_string())
}

/// Every bar the served month holds; `Ok(None)` for a month not held.
fn held_bars(root: &Path, path: StorePath<'_>, symbol: &str) -> Result<Option<Vec<Bar>>, String> {
    match store::file::BarFile::open_existing(
        root,
        path,
        pull::ingest::repair_plan::symbol_id(symbol),
    ) {
        Err(store::file::StoreError::Missing { .. }) => Ok(None),
        Err(why) => Err(why.to_string()),
        Ok(file) => (0..file.header().n_valid)
            .map(|index| file.read_record(index).map_err(|why| why.to_string()))
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
    }
}

/// Every bar the served month holds.
fn served_bars(root: &Path, path: StorePath<'_>, symbol: &str) -> Result<Vec<Bar>, String> {
    let id = pull::ingest::repair_plan::symbol_id(symbol);
    let file =
        store::file::BarFile::open_existing(root, path, id).map_err(|why| why.to_string())?;
    (0..file.header().n_valid)
        .map(|index| file.read_record(index).map_err(|why| why.to_string()))
        .collect()
}

fn ist_day(ts_micros: i64) -> Option<Day> {
    IstMoment::from_epoch_secs(ts_micros.div_euclid(1_000_000))
        .ok()
        .map(IstMoment::day)
}

/// The stamps of `row` inside the window, from the plan or (a whole month)
/// from the month as served.
fn named(root: &Path, ask: &Ask, row: &Row) -> Result<Vec<i64>, String> {
    let path = path_of(ask.segment, &row.symbol, row.rung, row.month)?;
    let stamps = match &row.stamps {
        Some(stamps) => stamps.clone(),
        None => served_bars(root, path, &row.symbol)?
            .iter()
            .map(|bar| bar.ts_micros)
            .collect(),
    };
    Ok(stamps
        .into_iter()
        .filter(|at| ist_day(*at).is_some_and(|day| ask.from <= day && day <= ask.to))
        .collect())
}

/// For `before_listing`, a stamp on or after the share's listing day is
/// refused by name: the plan was measured against a master, and this is the
/// same master asked once more before anything is written.
fn check_listing(root: &Path, ask: &Ask, symbol: &str, stamps: &[i64]) -> Result<(), String> {
    if ask.cause != Cause::BeforeListing {
        return Ok(());
    }
    let listing = pull::listing::for_store(root);
    let isin = brutex_core::universe::nse_isin(symbol);
    let isin = isin.as_ref().map(brutex_core::isin::Isin::as_str);
    let Some(span) = listing.span("NSE", symbol, isin) else {
        return Err(format!(
            "{symbol}: no master on disk states its listing day; a before_listing quarantine is refused"
        ));
    };
    let Some(listed) = span.listed_on() else {
        return Err(format!(
            "{symbol}: its master states no listing day; refused"
        ));
    };
    if let Some(after) = stamps
        .iter()
        .find(|at| ist_day(**at).is_none_or(|day| !span.before_listing(day)))
    {
        return Err(format!(
            "{symbol}: the plan names {after} (on or after its listing day {listed}) as before_listing; refused, nothing of this month written"
        ));
    }
    Ok(())
}

/// The derived rungs a minute month folds into.
fn derived_rungs() -> impl Iterator<Item = Timeframe> {
    Timeframe::KNOWN.iter().copied().filter(|rung| {
        rung.secs() > 60 && rung.secs().is_multiple_of(60) && *rung != Timeframe::DAY_1
    })
}

/// The stamps of `held` (bars of a rung `width` seconds wide, each stamped
/// at its open) whose span holds any of `minutes` (ascending). One merge.
pub(crate) fn covering(held: &[Bar], width_secs: u32, minutes: &[i64]) -> Vec<i64> {
    let width = i64::from(width_secs) * 1_000_000;
    let mut out = Vec::new();
    let mut next = minutes.iter().copied().peekable();
    for bar in held {
        while next.next_if(|at| *at < bar.ts_micros).is_some() {}
        if next.peek().is_some_and(|at| *at < bar.ts_micros + width) {
            out.push(bar.ts_micros);
        }
    }
    out
}

/// What one month came to.
enum Outcome {
    /// What `--plan` found: the bars named, and what the quarantine already
    /// says of the month (one probe), so a plan read after an apply says so.
    Planned(usize, Option<String>),
    Done(Applied),
    Refused(String),
}

fn day_groups(stamps: &[i64]) -> String {
    let mut groups: Vec<(Day, usize)> = Vec::new();
    for at in stamps {
        let Some(day) = ist_day(*at) else { continue };
        match groups.last_mut() {
            Some((last, count)) if *last == day => *count += 1,
            _ => groups.push((day, 1)),
        }
    }
    groups
        .iter()
        .map(|(day, count)| format!("{day} {count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn outcome_text(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Planned(count, None) => format!("plan: {count} bar(s) would be set aside"),
        Outcome::Planned(count, Some(already)) => {
            format!("plan: {count} bar(s) named; the month is already quarantined ({already})")
        }
        Outcome::Done(Applied::Nothing { why }) => format!("nothing written ({why})"),
        Outcome::Done(Applied::WholeMonth { bars, ledger_at }) => {
            format!(
                "RECORDED WHOLE: {bars} bar(s), no generation published (ledger at {ledger_at})"
            )
        }
        Outcome::Done(Applied::Dropped { drop, ledger_at }) => format!(
            "DROPPED {} of {} bar(s): generation {} -> {} served (ledger at {ledger_at})",
            drop.dropped,
            drop.held,
            drop.from,
            drop.to.ordinal()
        ),
        Outcome::Done(Applied::Resumed { bars, ledger_at }) => {
            format!(
                "RESUMED: {bars} bar(s) a killed run dropped, now recorded (ledger at {ledger_at})"
            )
        }
        Outcome::Refused(why) => format!("REFUSED: {why}"),
    }
}

fn note(ask: &Ask, symbol: &str, rung: Timeframe, month: YearMonth, outcome: &Outcome) {
    let month = month.to_string();
    let text = outcome_text(outcome);
    let event = match outcome {
        Outcome::Refused(_) => {
            telemetry::Event::warn(EVENT_TARGET, "zerodha quarantine refused a month")
        }
        _ => telemetry::Event::info(EVENT_TARGET, "zerodha quarantine month"),
    };
    crate::note(
        &event
            .with("symbol", telemetry::Value::Str(symbol))
            .with("rung", telemetry::Value::Str(rung.as_str()))
            .with("month", telemetry::Value::Str(&month))
            .with("cause", telemetry::Value::Str(ask.cause.code()))
            .with(
                "mode",
                telemetry::Value::Str(if ask.apply { "apply" } else { "plan" }),
            )
            .with("reason", telemetry::Value::Str(&ask.reason))
            .with("outcome", telemetry::Value::Str(&text)),
    );
}

/// The target of this verb's events on `/logs`.
const EVENT_TARGET: &str = "cli.zerodha_quarantine";

/// What the feed's quarantine says of itself before the first month: the
/// lines to print, or why nothing may be written. A ledger with a damaged
/// line is refused by name (its offsets); an apply then brings the index
/// level with the ledger (`store::quarantine::catch_up`), naming a rebuild.
/// O(ledger), once per run.
fn quarantine_state(root: &Path, apply: bool) -> Result<Vec<String>, String> {
    let ledger = store::quarantine::read_ledger(root, Vendor::Zerodha)
        .map_err(|why| format!("LEDGER UNREADABLE: {why}"))?;
    if !ledger.damaged.is_empty() {
        return Err(format!(
            "LEDGER DAMAGED: {}",
            store::quarantine::QuarantineError::LedgerDamaged {
                path: store::quarantine::ledger_path(root, Vendor::Zerodha),
                offsets: ledger.damaged,
            }
        ));
    }
    let mut said = Vec::new();
    if !ledger.ended.is_empty() {
        said.push(format!(
            "quarantine ledger: {} torn append(s) a killed writer left, ended and never read as acts (at byte {})",
            ledger.ended.len(),
            ledger
                .ended
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if apply {
        match store::quarantine::catch_up(root, Vendor::Zerodha) {
            Ok(None) => {}
            Ok(Some(rebuilt)) => said.push(format!(
                "QUARANTINE INDEX REBUILT from its ledger before the first act: {} line(s), {} month(s) indexed",
                rebuilt.lines, rebuilt.months
            )),
            Err(why) => return Err(format!("INDEX NOT LEVEL WITH ITS LEDGER: {why}")),
        }
    }
    Ok(said)
}

/// THE QUARANTINE'S OWN STATE FIRST (R4-18 repair 2), printed to `out`: a
/// damaged ledger line by offset, never skipped (a plan goes on and fails,
/// `Ok(true)`; an apply refuses before anything is written, `Err(FAILED)`),
/// and the rebuild an apply made to bring the index level with its ledger.
fn state_said(root: &Path, ask: &Ask, out: &mut String) -> Result<bool, u8> {
    match quarantine_state(root, ask.apply) {
        Ok(said) => {
            for line in &said {
                let _ = writeln!(out, "  {line}");
            }
            Ok(false)
        }
        Err(why) => {
            let _ = writeln!(out, "  QUARANTINE {why}");
            note_state(ask, &why);
            if ask.apply {
                let _ = writeln!(out, "ZERODHA QUARANTINE REFUSED. Nothing was written.");
                return Err(FAILED);
            }
            Ok(true)
        }
    }
}

/// One `/logs` event for a quarantine state that refused or failed a run.
fn note_state(ask: &Ask, why: &str) {
    crate::note(
        &telemetry::Event::error(EVENT_TARGET, "zerodha quarantine state refused")
            .with(
                "mode",
                telemetry::Value::Str(if ask.apply { "apply" } else { "plan" }),
            )
            .with("why", telemetry::Value::Str(why)),
    );
}

/// One month of one rung: planned, or applied.
fn act(
    root: &Path,
    ask: &Ask,
    symbol: &str,
    rung: Timeframe,
    month: YearMonth,
    stamps: &[i64],
) -> Outcome {
    let path = match path_of(ask.segment, symbol, rung, month) {
        Ok(path) => path,
        Err(why) => return Outcome::Refused(why),
    };
    if !ask.apply {
        let already = match store::quarantine::probe(root, path) {
            Ok(store::quarantine::Probe::Quarantined(entry)) => Some(format!(
                "{} {}, {} bar(s)",
                entry.scope.code(),
                entry.causes.said(),
                entry.bars
            )),
            Ok(store::quarantine::Probe::Clear) => None,
            Err(why) => Some(format!("not known: {why}")),
        };
        return Outcome::Planned(stamps.len(), already);
    }
    let reason = if rung == Timeframe::MINUTE_1 || rung == Timeframe::DAY_1 {
        ask.reason.clone()
    } else {
        let derived = format!(
            "{} (a {} bucket of a quarantined 1min bar)",
            ask.reason,
            rung.as_str()
        );
        if derived.len() <= store::quarantine::MAX_REASON {
            derived
        } else {
            ask.reason.clone()
        }
    };
    let why = Why {
        cause: ask.cause,
        who: "cli zerodha-quarantine",
        reason: &reason,
    };
    match store::quarantine::apply_month(
        root,
        path,
        pull::ingest::repair_plan::symbol_id(symbol),
        stamps,
        why,
    ) {
        Ok(applied) => Outcome::Done(applied),
        Err(why) => Outcome::Refused(why.to_string()),
    }
}

/// The plan's path and its rows for `ask`, or why not.
fn read_rows(root: &Path, ask: &Ask) -> Result<(PathBuf, Vec<Row>), String> {
    let path = plan_path(root, ask)?;
    let text =
        std::fs::read_to_string(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    let rows = rows_of(&text, ask).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok((path, rows))
}

/// Runs the verb against `store` (the writer's gate), writing to `out`.
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
                "ZERODHA QUARANTINE REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    let (plan, rows) = match read_rows(&root, &ask) {
        Ok(read) => read,
        Err(why) => {
            let _ = writeln!(
                out,
                "ZERODHA QUARANTINE REFUSED: {why}. Nothing was written."
            );
            return FAILED;
        }
    };
    let _ = writeln!(
        out,
        "zerodha-quarantine {} {} {}..{} cause {} ({}), plan {}",
        ask.segment,
        ask.symbol.as_deref().unwrap_or("ALL"),
        ask.from,
        ask.to,
        ask.cause.code(),
        if ask.apply {
            "APPLY"
        } else {
            "plan only: nothing is written"
        },
        plan.display()
    );
    let mut failed = match state_said(&root, &ask, out) {
        Ok(failed) => failed,
        Err(code) => return code,
    };
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "  the plan names no bar of this cause in the window: nothing to do"
        );
        return if failed { FAILED } else { OK };
    }
    for row in &rows {
        let stamps = match named(&root, &ask, row)
            .and_then(|stamps| check_listing(&root, &ask, &row.symbol, &stamps).map(|()| stamps))
        {
            Ok(stamps) if stamps.is_empty() => continue,
            Ok(stamps) => stamps,
            Err(why) => {
                let outcome = Outcome::Refused(why);
                let _ = writeln!(
                    out,
                    "  {} {} {}: {}",
                    row.symbol,
                    row.rung.as_str(),
                    row.month,
                    outcome_text(&outcome)
                );
                note(&ask, &row.symbol, row.rung, row.month, &outcome);
                failed = true;
                continue;
            }
        };
        let outcome = act(&root, &ask, &row.symbol, row.rung, row.month, &stamps);
        failed |= matches!(outcome, Outcome::Refused(_));
        let _ = writeln!(
            out,
            "  {} {} {}: {} {} bar(s) [{}] - {}",
            row.symbol,
            row.rung.as_str(),
            row.month,
            ask.cause.code(),
            stamps.len(),
            day_groups(&stamps),
            outcome_text(&outcome)
        );
        note(&ask, &row.symbol, row.rung, row.month, &outcome);
        if row.rung != Timeframe::MINUTE_1 || matches!(outcome, Outcome::Refused(_)) {
            continue;
        }
        failed |= derived(&root, &ask, row, &stamps, out);
    }
    if failed { FAILED } else { OK }
}

/// A minute month's derived rungs set aside after it: every bucket whose span
/// holds a named minute, with the same cause. Answers whether one refused.
fn derived(root: &Path, ask: &Ask, row: &Row, stamps: &[i64], out: &mut String) -> bool {
    let mut failed = false;
    for rung in derived_rungs() {
        let Ok(path) = path_of(ask.segment, &row.symbol, rung, row.month) else {
            continue;
        };
        let held = match held_bars(root, path, &row.symbol) {
            Ok(Some(held)) => held,
            // A DERIVED MONTH THAT IS NOT THERE has nothing to set aside.
            Ok(None) => continue,
            Err(why) => {
                let outcome = Outcome::Refused(why);
                let _ = writeln!(
                    out,
                    "    {} {}: {}",
                    rung.as_str(),
                    row.month,
                    outcome_text(&outcome)
                );
                note(ask, &row.symbol, rung, row.month, &outcome);
                failed = true;
                continue;
            }
        };
        let buckets = covering(&held, rung.secs(), stamps);
        if buckets.is_empty() {
            continue;
        }
        let outcome = act(root, ask, &row.symbol, rung, row.month, &buckets);
        failed |= matches!(outcome, Outcome::Refused(_));
        let _ = writeln!(
            out,
            "    {} {}: {} bucket(s) hold a named minute - {}",
            rung.as_str(),
            row.month,
            buckets.len(),
            outcome_text(&outcome)
        );
        note(ask, &row.symbol, rung, row.month, &outcome);
    }
    failed
}

/// The verb over a root the caller already holds (the API's
/// `/pull/quarantine` routes, R4-18): an `--apply` is refused on a store
/// whose `bars` is a link, as every writer's is (`store_write::writable`).
pub(crate) fn command_at(arguments: &[&str], root: &Path, out: &mut String) -> u8 {
    let root = root.to_path_buf();
    // THE PARSED MODE, not a token search: `--reason --apply` names a
    // reason, and a plan carrying it must not take the writer's gate.
    let apply = ask(arguments).is_ok_and(|ask| ask.apply);
    command_with(
        arguments,
        &move || {
            if apply {
                crate::store_write::writable(&root)?;
            }
            Ok(root.clone())
        },
        out,
    )
}

/// `cli zerodha-quarantine`: see the module header. A STORE WRITER with
/// `--apply`: the root comes through the writer's gate.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(
        arguments,
        &crate::store_write::root_for("zerodha-quarantine", arguments),
        out,
    )
}

#[cfg(test)]
#[path = "zerodha_quarantine_tests.rs"]
mod tests;
