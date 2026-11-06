//! The golden-file writer: one plain-text Markdown file per scenario holding its inputs
//! (bars and seconds) and every expected output (each candidate's fills and charges,
//! every row's statistics, counters and months, and the rank), format
//! `psrb-ref-golden/1`. Files are written with `create_new` only: an existing file is
//! compared, never overwritten, never deleted.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

use crate::book::View;
use crate::candles::TIMEFRAMES;
use crate::costs::{Clock, Lines};
use crate::generator::Scenario;
use crate::market::{Bar, DayKind, Market, Stock};
use crate::membership::Answer;
use crate::readings::KINDS;
use crate::ruleset::RULE_SETS;
use crate::run::{Output, Record, Row, RunError, run};
use crate::time::{Day, Month, hm, hms};
use crate::trade::Outcome;
use crate::verdict::{Rankable, rank};

/// The format line every golden file carries.
pub const FORMAT: &str = "psrb-ref-golden/1";
/// The session token every golden file names.
pub const TOKEN: &str = "PSRB-PREVIEW-REF-20261106-1";

fn kind_word(kind: DayKind) -> &'static str {
    match kind {
        DayKind::Full => "full",
        DayKind::Short => "short",
        DayKind::Muhurat => "muhurat",
        DayKind::Closed => "closed",
    }
}

fn answer_word(a: Answer) -> &'static str {
    match a {
        Answer::Yes => "yes",
        Answer::No => "no",
        Answer::Unknown => "unknown",
    }
}

fn months_line(label: &str, months: &mut dyn Iterator<Item = Month>) -> String {
    let list: Vec<String> = months.map(Month::label).collect();
    format!("{label} {}", if list.is_empty() { "-".to_owned() } else { list.join(" ") })
}

fn bar_text(b: &Bar) -> String {
    format!("{} {} {} {} {}", b.open, b.high, b.low, b.close, b.volume)
}

/// Run-length lines: `(start, bar)` items whose starts step by `stride` with one bar
/// become one line `tag time bar x<count> [every <stride>]`.
fn runs(out: &mut String, tag: &str, items: &[(u32, Bar)], fixed_stride: Option<u32>, time: fn(u32) -> String) {
    let mut i = 0;
    while let Some((start, bar)) = items.get(i).copied() {
        let next = items.get(i + 1).copied();
        let stride = fixed_stride.or_else(|| next.map(|(s, _)| s.saturating_sub(start)));
        let mut count = 1;
        let mut at = start;
        while let Some((s, b)) = items.get(i + count).copied() {
            if b != bar || stride != Some(s.saturating_sub(at)) {
                break;
            }
            at = s;
            count += 1;
        }
        let tail = match (count, fixed_stride) {
            (1, _) => String::new(),
            (_, Some(_)) => format!(" x{count}"),
            (_, None) => format!(" x{count} every {}", stride.unwrap_or(0)),
        };
        let _ = writeln!(out, "{tag} {} {}{tail}", time(start), bar_text(&bar));
        i += count;
    }
}

fn render_stock(out: &mut String, stock: &Stock) {
    let _ = writeln!(out, "stock {}", stock.name);
    let _ = writeln!(out, "{}", months_line("zerodha-months", &mut stock.zerodha_months.iter().copied()));
    let _ = writeln!(out, "{}", months_line("gdfl-months", &mut stock.gdfl_months.iter().copied()));
    let _ = writeln!(out, "{}", months_line("gdfl-refused", &mut stock.gdfl_refused.iter().copied()));
    let withheld: Vec<String> = stock.withheld.iter().map(|d| d.label()).collect();
    let _ = writeln!(out, "withheld {}", if withheld.is_empty() { "-".to_owned() } else { withheld.join(" ") });
    for (day, sd) in &stock.days {
        let _ = writeln!(out, "day {} minutes={} seconds={}", day.label(), sd.minutes.len(), sd.seconds.len());
        let minutes: Vec<(u32, Bar)> = sd.minutes.iter().map(|m| (m.start, m.bar)).collect();
        runs(out, "m", &minutes, Some(60), hm_or_hms);
        let seconds: Vec<(u32, Bar)> = sd.seconds.iter().map(|s| (s.at, s.bar)).collect();
        runs(out, "s", &seconds, None, hms);
    }
    let _ = writeln!(out, "end-stock");
}

/// A minute stamp as `hh:mm`, or `hh:mm:ss` when it is off the minute grid.
fn hm_or_hms(sec: u32) -> String {
    if sec.is_multiple_of(60) { hm(sec) } else { hms(sec) }
}

/// The inputs section's body.
#[must_use]
pub fn render_inputs(market: &Market) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "window {} {}", market.window.first.label(), market.window.last.label());
    let _ = writeln!(out, "calendar weekdays=full weekends=closed");
    for (day, kind) in &market.calendar.overrides {
        let _ = writeln!(out, "calendar-override {} {}", day.label(), kind_word(*kind));
    }
    let p = &market.membership.probe;
    let _ = writeln!(
        out,
        "membership-probe opened={} reliance-2018-09-03={} reliance-2026-09-30={} syrma-2026-09-30={}",
        if p.opened { "yes" } else { "no" },
        answer_word(p.reliance_first),
        answer_word(p.reliance_last),
        answer_word(p.syrma_last)
    );
    let carry = market.membership.carry_from.map_or_else(|| "-".to_owned(), Day::label);
    let _ = writeln!(out, "membership-carry-from {carry}");
    let listed: Vec<&str> = market.membership.listed.iter().map(String::as_str).collect();
    let _ = writeln!(out, "membership-listed {}", listed.join(" "));
    for ((name, day), answer) in &market.membership.answers {
        let _ = writeln!(out, "membership-answer {name} {} {}", day.label(), answer_word(*answer));
    }
    let mut stocks: Vec<&Stock> = market.stocks.iter().collect();
    stocks.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    for stock in stocks {
        render_stock(&mut out, stock);
    }
    out
}

fn lines_text(l: &Lines) -> String {
    let parts: Vec<String> = l.as_array().iter().map(i128::to_string).collect();
    format!("{} {}", parts.join(" "), l.total())
}

fn opt(v: Option<i128>) -> String {
    v.map_or_else(|| "n/a".to_owned(), |x| x.to_string())
}

/// One candidate line.
fn candidate_line(id: usize, r: &Record) -> String {
    let c = &r.candidate;
    let head = format!("{id} {} {} {} {}", c.day.label(), hms(c.t), c.symbol, c.side.label());
    let Some(t) = r.trade else {
        let word = match c.outcome {
            Outcome::NotFilled => "not-filled",
            Outcome::NoRatio => "no-ratio",
            _ => "zero-quantity",
        };
        return format!("{head} {word}");
    };
    let s = t.sim;
    let from = if t.priced.from == Clock::Dated { "dated" } else { "today" };
    format!(
        "{head} filled entry={} {} qty={} exit={} {} kind={} fallback={} late={} stop={}..{} trail-no-ratio={} gross={} headline={} from={from} net={} | dated-worst {} | today-worst {} | dated-precise {} | today-precise {}",
        hms(s.entry.second),
        s.entry.price,
        s.qty,
        hms(s.exit.second),
        s.exit.price,
        s.kind.label(),
        u8::from(s.fallback),
        u8::from(s.late),
        s.first_stop,
        s.last_stop,
        s.no_ratio_trail,
        t.gross,
        t.priced.headline,
        t.net,
        lines_text(&t.priced.dated_worst),
        lines_text(&t.priced.today_worst),
        t.priced.dated_precise.total(),
        t.priced.today_precise.total(),
    )
}

fn per_kind(counts: &[u64; 6]) -> String {
    let parts: Vec<String> = KINDS.iter().zip(counts.iter()).map(|(k, c)| format!("{}:{c}", k.label())).collect();
    parts.join(",")
}

fn row_lines(out: &mut String, row: &Row) {
    let set = RULE_SETS.get(row.key.set).map_or("?", |s| s.short);
    let s = &row.stats;
    let dd = &row.drawdown;
    let peak = dd.peak_day.map_or_else(|| "start".to_owned(), Day::label);
    let low = dd.low_day.map_or_else(|| "none".to_owned(), Day::label);
    let _ = writeln!(
        out,
        "row {set} {} {} verdict={} trades={} net={} net-bp={} dd-bp={} dd={} dd-peak={peak} dd-low={low} months={} months-pass={} months-positive={} worst={} median={} best={} floor={} win-rate={} avg-win={} avg-loss={} pf={} charges={}",
        row.key.k,
        row.key.view.label(),
        if row.bar.pass { "PASS" } else { "MISS" },
        s.trades,
        s.total_net,
        s.total_bp,
        dd.bp,
        dd.paisa,
        s.months,
        s.months_pass,
        s.months_positive,
        opt(s.worst_bp),
        opt(s.median_bp),
        opt(s.best_bp),
        opt(row.bar.floor),
        opt(s.win_rate),
        opt(s.avg_win),
        opt(s.avg_loss),
        opt(s.profit_factor),
        s.charges,
    );
    let c = &row.counts;
    let _ = writeln!(
        out,
        "counts signals={} while-open={} cut={} not-filled={} zero-qty={} no-ratio={} late={} stop={} flip={} time={} square-off={} unclosable={} slots-full={} fallback-days={} no-ratio-trail={} cold={} taint={}",
        c.signals,
        c.while_open,
        c.cut,
        c.not_filled,
        c.zero_qty,
        c.no_ratio,
        c.late,
        c.exits[0],
        c.exits[1],
        c.exits[2],
        c.exits[3],
        c.exits[4],
        c.slots_full,
        c.fallback_days,
        c.no_ratio_trail,
        per_kind(&c.blocked_cold),
        per_kind(&c.blocked_taint),
    );
    let ids: Vec<String> = row.admitted.iter().map(usize::to_string).collect();
    let _ = writeln!(out, "admitted {}", if ids.is_empty() { "-".to_owned() } else { ids.join(" ") });
    for (i, m) in row.months.iter().enumerate() {
        let gap = row.bar.gaps.iter().find(|g| g.index == i).map_or_else(|| "-".to_owned(), |g| g.exact.to_string());
        let _ = writeln!(
            out,
            "month {} capital={} net={} trades={} bp={} pass={} gap={gap}",
            m.month.label(),
            m.capital_start,
            m.net,
            m.trades,
            opt(m.bp),
            if m.pass { "yes" } else { "no" },
        );
    }
}

/// The expected-outputs section's body.
#[must_use]
pub fn render_outputs(out_run: &Output) -> String {
    let mut out = String::new();
    for f in &out_run.stocks {
        let span = f.span.map_or_else(|| "none".to_owned(), |(a, b)| format!("{}..{}", a.label(), b.label()));
        let warm = f.warmup.map_or_else(|| "none".to_owned(), Month::label);
        let refused = f.refused.map_or_else(|| "none".to_owned(), |r| format!("{r:?}"));
        let errors: Vec<String> = f.fold_errors.iter().map(|m| m.label()).collect();
        let _ = writeln!(
            out,
            "stock {} span={span} warmup={warm} refused={refused} fold-errors={} membership-unknown={}",
            f.name,
            if errors.is_empty() { "-".to_owned() } else { errors.join(",") },
            f.membership_unknown
        );
        for (k, fc) in TIMEFRAMES.iter().zip(f.fold.iter()) {
            let _ = writeln!(out, "fold {} k={k} holes={} outside-session={}", f.name, fc.holes, fc.outside_session);
        }
    }
    for ((set, k), list) in &out_run.records {
        let short = RULE_SETS.get(*set).map_or("?", |s| s.short);
        let _ = writeln!(out, "candidates {short} {k} count={}", list.len());
        for (id, r) in list.iter().enumerate() {
            let _ = writeln!(out, "{}", candidate_line(id, r));
        }
    }
    for row in &out_run.rows {
        row_lines(&mut out, row);
    }
    let rankables: Vec<Rankable> = out_run
        .rows
        .iter()
        .map(|r| Rankable {
            pass: r.bar.pass,
            max_dd_bp: r.drawdown.bp,
            total_net: r.stats.total_net,
            win_rate: r.stats.win_rate.unwrap_or(0),
            months_below: r.bar.gaps.len(),
            worst: r.bar.floor,
            tie: (r.key.set, r.key.k, View::ALL.iter().position(|v| *v == r.key.view).unwrap_or(0)),
        })
        .collect();
    for (place, r) in rank(&rankables).iter().filter_map(|i| out_run.rows.get(*i)).enumerate() {
        let set = RULE_SETS.get(r.key.set).map_or("?", |s| s.short);
        let _ = writeln!(out, "rank {} {set} {} {}", place + 1, r.key.k, r.key.view.label());
    }
    out
}

/// One scenario's whole golden file.
#[must_use]
pub fn render(scenario: &Scenario) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Golden scenario `{}`", scenario.name);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "PREVIEW: a synthetic reference scenario for differential testing of the PSRB preview tool; not the PSRB result; every price and trade here is generated with a fixed seed."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "- format: `{FORMAT}`; written by crate `psrb_preview_ref`, session token `{TOKEN}`");
    let _ = writeln!(out, "- seed: `{:#018x}`", scenario.seed);
    let _ = writeln!(out, "- plants: {}", scenario.about);
    let _ = writeln!(out, "- money in paisa; times IST; `x<n>` repeats a line n times (minutes every 60 s, seconds every `every` s)");
    let _ = writeln!(out);
    let _ = writeln!(out, "## Inputs");
    let _ = writeln!(out);
    let _ = writeln!(out, "```text");
    out.push_str(&render_inputs(&scenario.market));
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(out, "## Expected outputs");
    let _ = writeln!(out);
    let _ = writeln!(out, "```text");
    match run(&scenario.market) {
        Ok(output) => out.push_str(&render_outputs(&output)),
        Err(e) => out.push_str(&render_error(&e)),
    }
    let _ = writeln!(out, "```");
    out
}

/// A run that stopped by name.
#[must_use]
pub fn render_error(e: &RunError) -> String {
    match e {
        RunError::Charge { symbol, day, error } => format!("run-stopped charge-refusal {symbol} {} {error:?}\n", day.label()),
        RunError::Duplicate(d) => format!("run-stopped duplicate-candidate {} {} {}\n", d.symbol, d.day.label(), hms(d.t)),
    }
}

/// What writing one golden file found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The file was new and is now written.
    Written,
    /// The file existed with the same bytes.
    Same,
    /// The file existed with other bytes (left untouched).
    Differs,
}

/// Writes `<dir>/<name>.txt.md` for each scenario with `create_new`; an existing file is
/// compared and never overwritten.
///
/// # Errors
/// An I/O error creating, writing or reading a file.
pub fn write_all(dir: &Path, scenarios: &[Scenario]) -> std::io::Result<Vec<(String, Status)>> {
    std::fs::create_dir_all(dir)?;
    let mut out = Vec::with_capacity(scenarios.len());
    for s in scenarios {
        let path = dir.join(format!("{}.txt.md", s.name));
        let text = render(s);
        let status = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())?;
                file.sync_all()?;
                Status::Written
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if std::fs::read(&path)? == text.as_bytes() { Status::Same } else { Status::Differs }
            }
            Err(e) => return Err(e),
        };
        out.push((s.name.clone(), status));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "golden_tests.rs"]
mod tests;
