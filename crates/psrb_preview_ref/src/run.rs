//! The run: Phase A per stock and timeframe (fold, readings, signals and each signal's
//! portfolio-free trade, priced) and Phase B per row (rule set x timeframe x account:
//! slots, months, drawdown, statistics and the verdict). PREVIEW-SPEC section 5.

use std::collections::{BTreeMap, BTreeSet};

use crate::account::{Closed, Drawdown, MonthRow, drawdown, months};
use crate::book::{Admission, Candidate, DuplicateKey, Slots, View, merge};
use crate::candles::{FoldCounts, TIMEFRAMES, check_minutes, fold_day, session_close};
use crate::cond::Side;
use crate::costs::{ChargeError, Priced, gross_and_net, price_trade, trade_orders};
use crate::daymask::{Facts, Verdict, admit};
use crate::dayref::{PrevSession, day_bar};
use crate::indicators::supertrend::Reading as StReading;
use crate::market::{DayKind, Market, Stock};
use crate::membership::Answer;
use crate::readings::{Bank, KINDS, Step};
use crate::report::{RowStats, row_stats};
use crate::ruleset::{RULE_SETS, RuleSet, supertrend_factor};
use crate::signal::{Blocker, Context, evaluate, ready_gate};
use crate::span::{Refusal as SpanRefusal, trading_span, warmup_start};
use crate::time::{Day, ENTRY_CUT, Month};
use crate::trade::{ExitKind, Later, Outcome, Simulated, Tape, simulate};
use crate::verdict::{Bar, bar};

/// A priced, filled would-be trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PricedTrade {
    /// The simulated fills.
    pub sim: Simulated,
    /// Both clocks' notes and the headline.
    pub priced: Priced,
    /// Sell value minus buy value, paisa.
    pub gross: i128,
    /// Gross minus the headline.
    pub net: i128,
}

/// One candidate and, when it filled, its priced trade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The candidate.
    pub candidate: Candidate,
    /// Its priced trade.
    pub trade: Option<PricedTrade>,
}

/// A row's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowKey {
    /// Index into [`RULE_SETS`].
    pub set: usize,
    /// The timeframe in minutes.
    pub k: u32,
    /// The account.
    pub view: View,
}

/// Every counter of one row (SPEC section 3.10, "also").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Signal candles (cut ones included).
    pub signals: u64,
    /// Signals while the stock held a position.
    pub while_open: u64,
    /// Signals at or after 15:08:00.
    pub cut: u64,
    /// Entries with no traded unit left that day.
    pub not_filled: u64,
    /// Entries that bought less than one share.
    pub zero_qty: u64,
    /// Entries with no ratio minute for the stop.
    pub no_ratio: u64,
    /// Admitted trades whose entry filled more than 60 s after T.
    pub late: u64,
    /// Admitted trades by exit kind (stop, flip, time, square-off, unclosable).
    pub exits: [u64; 5],
    /// Signals skipped with 20 positions open.
    pub slots_full: u64,
    /// Stock-days on which an admitted trade filled on minutes.
    pub fallback_days: u64,
    /// Admitted trades' trailing moves that could not be mapped.
    pub no_ratio_trail: u64,
    /// Decisions blocked by a cold reading, per reading kind (in table order).
    pub blocked_cold: [u64; 6],
    /// Decisions blocked by a tainted reading, per reading kind.
    pub blocked_taint: [u64; 6],
}

/// One row's result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Its key.
    pub key: RowKey,
    /// Indices of its admitted trades in the (rule set, timeframe) record list.
    pub admitted: Vec<usize>,
    /// Its months.
    pub months: Vec<MonthRow>,
    /// Its max drawdown.
    pub drawdown: Drawdown,
    /// Its statistics.
    pub stats: RowStats,
    /// Its verdict.
    pub bar: Bar,
    /// Its counters.
    pub counts: Counts,
}

/// Per-stock facts of the run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StockFacts {
    /// The stock.
    pub name: String,
    /// Its trading span, if any.
    pub span: Option<(Day, Day)>,
    /// Its warm-up start month.
    pub warmup: Option<Month>,
    /// A refusal of its month lists (counted; the stock does not trade).
    pub refused: Option<SpanRefusal>,
    /// Months whose minutes the fold refused.
    pub fold_errors: Vec<Month>,
    /// Full-session days in the span with membership unknown.
    pub membership_unknown: u64,
    /// Fold counters per timeframe, in [`TIMEFRAMES`] order.
    pub fold: [FoldCounts; 6],
}

/// Everything a run computes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// Per (rule set, timeframe): every candidate in (day, T, symbol) order, priced.
    pub records: BTreeMap<(usize, u32), Vec<Record>>,
    /// The 54 rows (rule set x timeframe x account), in key order.
    pub rows: Vec<Row>,
    /// Per-stock facts, in symbol byte order.
    pub stocks: Vec<StockFacts>,
}

/// A refusal that stops the run by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// `price_trade` refused a trade.
    Charge {
        /// The stock.
        symbol: String,
        /// The trade day.
        day: Day,
        /// The refusal.
        error: ChargeError,
    },
    /// One (day, T, symbol) twice.
    Duplicate(DuplicateKey),
}

/// One bucket of a stock-day as the trade walk reads it.
#[derive(Clone, Copy, Debug)]
struct DayCandle {
    t: u32,
    close: i64,
    hole: bool,
    st2: Option<StReading>,
    st3: Option<StReading>,
}

impl DayCandle {
    fn band(&self, factor: u32, side: Side) -> i128 {
        let reading = if factor == 2_000 { self.st2 } else { self.st3 };
        reading.map_or(0, |r| match side {
            Side::Long => r.lower,
            Side::Short => r.upper,
        })
    }
}

/// Cut signals and blocked decisions of one (rule set, timeframe).
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    cut_long: u64,
    cut_short: u64,
    cold: [u64; 6],
    taint: [u64; 6],
}

/// Adds one to the counter of `kind` in a per-reading array.
fn bump(counts: &mut [u64; 6], kind: crate::readings::Kind) {
    counts.iter_mut().zip(KINDS).filter(|(_, k)| *k == kind).for_each(|(c, _)| *c += 1);
}

/// One signal waiting for its trade walk.
struct Signal {
    set: usize,
    rule: RuleSet,
    side: Side,
    at: usize,
    candle: DayCandle,
}

/// The facts of a stock's trading span, or the refusal that leaves it out.
fn spans(market: &Market, stock: &Stock, facts: &mut StockFacts) -> Option<((Day, Day), Month)> {
    let zerodha: Vec<Month> = stock.zerodha_months.iter().copied().collect();
    let gdfl: Vec<Month> = stock.gdfl_months.iter().copied().collect();
    let span = match trading_span(&zerodha, &gdfl, &market.window, &market.calendar) {
        Ok(found) => found?,
        Err(why) => {
            facts.refused = Some(why);
            return None;
        }
    };
    facts.span = Some(span);
    match warmup_start(&zerodha, span.0.month()) {
        Ok(warm) => {
            facts.warmup = Some(warm);
            Some((span, warm))
        }
        Err(why) => {
            facts.refused = Some(why);
            None
        }
    }
}

/// Phase A for one stock: every candidate of every (rule set, timeframe).
fn phase_a(
    market: &Market,
    stock: &Stock,
    records: &mut BTreeMap<(usize, u32), Vec<Record>>,
    tallies: &mut BTreeMap<(usize, u32), Tally>,
) -> Result<StockFacts, RunError> {
    let mut facts = StockFacts { name: stock.name.clone(), ..StockFacts::default() };
    let Some((span, warm)) = spans(market, stock, &mut facts) else {
        return Ok(facts);
    };
    let days = market.calendar.full_days(warm.start(), span.1);
    let refused: BTreeSet<Month> =
        stock.days.iter().filter(|(day, sd)| check_minutes(**day, &sd.minutes).is_err()).map(|(day, _)| day.month()).collect();
    facts.fold_errors = refused.iter().copied().collect();
    let (w_first, w_last) = (market.window.first.start(), market.window.last.end());
    let mut fold = [FoldCounts::default(); 6];
    for (ki, (k, fold_k)) in TIMEFRAMES.iter().copied().zip(fold.iter_mut()).enumerate() {
        let mut bank = Bank::new();
        let mut prev = PrevSession::default();
        for day in days.iter().copied() {
            let sd = stock.days.get(&day);
            let minutes = sd.map_or(&[][..], |s| s.minutes.as_slice());
            let seconds = sd.map_or(&[][..], |s| s.seconds.as_slice());
            let blocked = stock.withheld.contains(&day) || refused.contains(&day.month());
            let buckets = fold_day(day, minutes, k, blocked, fold_k);
            let in_window = day >= w_first && day <= w_last;
            let member = if in_window {
                market.membership.may_trade(&stock.name, day, w_first, w_last).unwrap_or(Answer::No)
            } else {
                Answer::Yes
            };
            let in_span = day >= span.0 && day <= span.1;
            if ki == 0 && in_span && member == Answer::Unknown {
                facts.membership_unknown += 1;
            }
            let listed = market.membership.listed.contains(&stock.name);
            let has_minutes = !minutes.is_empty();
            let verdict = admit(day, &Facts { listed, kind: DayKind::Full, member, in_span, has_minutes });
            let trading = !matches!(verdict, Verdict::NoTrade(_));
            let month = day.month();
            let fallback = verdict == Verdict::TradeFallback
                || !stock.gdfl_months.contains(&month)
                || stock.gdfl_refused.contains(&month)
                || (seconds.is_empty() && has_minutes);
            let reference = prev.reference();
            let mut candles: Vec<DayCandle> = Vec::with_capacity(buckets.len());
            let mut signals: Vec<Signal> = Vec::new();
            for b in &buckets {
                let clean = bank.step(b.bar.as_ref(), b.hole) == Step::Clean;
                let close = b.bar.map_or(0, |bar| bar.close);
                let candle = DayCandle { t: b.t, close, hole: !clean, st2: bank.st2.reading(), st3: bank.st3.reading() };
                candles.push(candle);
                if !clean {
                    continue;
                }
                for (set, rule) in RULE_SETS.iter().enumerate() {
                    let ctx = Context { bank: &bank, close, prev: reference, in_span: trading, hole: false };
                    let tally = tallies.entry((set, k)).or_default();
                    for blocker in ready_gate(rule, &ctx) {
                        match blocker {
                            Blocker::Cold(kind) if trading => bump(&mut tally.cold, kind),
                            Blocker::Tainted(kind) if trading => bump(&mut tally.taint, kind),
                            _ => {}
                        }
                    }
                    if let Some(side) = evaluate(rule, &ctx) {
                        signals.push(Signal { set, rule: *rule, side, at: candles.len() - 1, candle });
                    }
                }
            }
            let tape = Tape { minutes, seconds, fallback, close: session_close(day, minutes) };
            for s in signals {
                let tally = tallies.entry((s.set, k)).or_default();
                if s.candle.t >= ENTRY_CUT {
                    if s.side == Side::Long {
                        tally.cut_long += 1;
                    } else {
                        tally.cut_short += 1;
                    }
                    continue;
                }
                let factor = supertrend_factor(&s.rule).unwrap_or(2_000);
                let later: Vec<Later> = candles
                    .iter()
                    .skip(s.at + 1)
                    .map(|c| Later { t: c.t, close: c.close, band: c.band(factor, s.side), hole: c.hole })
                    .collect();
                let outcome = simulate(s.side, s.candle.t, s.candle.band(factor, s.side), &later, &tape);
                let trade = price(&stock.name, day, s.side, outcome)?;
                let candidate = Candidate { day, t: s.candle.t, symbol: stock.name.clone(), side: s.side, outcome };
                records.entry((s.set, k)).or_default().push(Record { candidate, trade });
            }
            prev.close_day(if blocked { None } else { day_bar(day, minutes) });
        }
    }
    facts.fold = fold;
    Ok(facts)
}

/// Prices a filled outcome (a refusal stops the run by name).
fn price(symbol: &str, day: Day, side: Side, outcome: Outcome) -> Result<Option<PricedTrade>, RunError> {
    let Outcome::Filled(sim) = outcome else {
        return Ok(None);
    };
    let long = side == Side::Long;
    let orders = trade_orders(long, sim.qty, sim.entry.price, sim.exit.price, sim.kind.by_broker());
    let priced =
        price_trade(&orders, day).map_err(|error| RunError::Charge { symbol: symbol.to_owned(), day, error })?;
    let (gross, net) = gross_and_net(long, sim.qty, sim.entry.price, sim.exit.price, priced.headline);
    Ok(Some(PricedTrade { sim, priced, gross, net }))
}

/// Phase B for one row.
fn phase_b(market: &Market, key: RowKey, records: &[Record], tally: Tally) -> Row {
    let cut = match key.view {
        View::Long => tally.cut_long,
        View::Short => tally.cut_short,
        View::Both => tally.cut_long + tally.cut_short,
    };
    let mut counts =
        Counts { cut, signals: cut, blocked_cold: tally.cold, blocked_taint: tally.taint, ..Counts::default() };
    let mut slots = Slots::default();
    let mut admitted = Vec::new();
    for (i, r) in records.iter().enumerate().filter(|(_, r)| key.view.takes(r.candidate.side)) {
        counts.signals += 1;
        match slots.offer(&r.candidate) {
            Admission::Admitted => admitted.push(i),
            Admission::WhileOpen => counts.while_open += 1,
            Admission::SlotsFull => counts.slots_full += 1,
            Admission::Unfillable => match r.candidate.outcome {
                Outcome::NotFilled => counts.not_filled += 1,
                Outcome::NoRatio => counts.no_ratio += 1,
                _ => counts.zero_qty += 1,
            },
        }
    }
    let mut trades: Vec<(&Record, PricedTrade)> =
        admitted.iter().filter_map(|i| records.get(*i)).filter_map(|r| r.trade.map(|t| (r, t))).collect();
    let mut fallback_days = BTreeSet::new();
    for (r, t) in &trades {
        counts.late += u64::from(t.sim.late);
        counts.no_ratio_trail += u64::from(t.sim.no_ratio_trail);
        counts.exits.iter_mut().zip(ExitKind::ALL).filter(|(_, k)| *k == t.sim.kind).for_each(|(c, _)| *c += 1);
        if t.sim.fallback {
            fallback_days.insert((r.candidate.symbol.as_str(), r.candidate.day));
        }
    }
    counts.fallback_days = u64::try_from(fallback_days.len()).unwrap_or(u64::MAX);
    trades.sort_by(|a, b| {
        let ka = (a.0.candidate.day, a.1.sim.exit.second, a.0.candidate.symbol.as_bytes());
        ka.cmp(&(b.0.candidate.day, b.1.sim.exit.second, b.0.candidate.symbol.as_bytes()))
    });
    let marks: Vec<Closed> =
        trades.iter().map(|(r, t)| Closed { day: r.candidate.day, second: t.sim.exit.second, net: t.net }).collect();
    let month_rows = months(&market.window, &marks);
    let nets: Vec<i128> = trades.iter().map(|(_, t)| t.net).collect();
    let charges: i128 = trades.iter().map(|(_, t)| t.priced.headline).sum();
    let stats = row_stats(&month_rows, &nets, charges);
    Row { key, admitted, drawdown: drawdown(&marks), bar: bar(&month_rows), months: month_rows, stats, counts }
}

/// Runs the whole recipe on a market.
///
/// # Errors
/// [`RunError`]: a trade `price_trade` refuses, or a duplicate candidate key (two stocks
/// with one spelling).
pub fn run(market: &Market) -> Result<Output, RunError> {
    let mut stocks: Vec<&Stock> = market.stocks.iter().collect();
    stocks.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    let mut per_stock: BTreeMap<(usize, u32), Vec<Vec<Candidate>>> = BTreeMap::new();
    let mut all: BTreeMap<(usize, u32), Vec<Record>> = BTreeMap::new();
    let mut tallies: BTreeMap<(usize, u32), Tally> = BTreeMap::new();
    let mut facts = Vec::new();
    for stock in stocks {
        let mut own = BTreeMap::new();
        facts.push(phase_a(market, stock, &mut own, &mut tallies)?);
        for (key, list) in own {
            per_stock.entry(key).or_default().push(list.iter().map(|r: &Record| r.candidate.clone()).collect());
            all.entry(key).or_default().extend(list);
        }
    }
    let mut records = BTreeMap::new();
    for (key, mut list) in all {
        let lists = per_stock.get(&key).map_or(&[][..], Vec::as_slice);
        merge(lists, View::Both).map_err(RunError::Duplicate)?;
        list.sort_by(|a, b| a.candidate.key().cmp(&b.candidate.key()));
        records.insert(key, list);
    }
    let mut rows = Vec::new();
    for set in 0..RULE_SETS.len() {
        for k in TIMEFRAMES {
            for view in View::ALL {
                let list = records.get(&(set, k)).map_or(&[][..], Vec::as_slice);
                let tally = tallies.get(&(set, k)).copied().unwrap_or_default();
                rows.push(phase_b(market, RowKey { set, k, view }, list, tally));
            }
        }
    }
    Ok(Output { records, rows, stocks: facts })
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
