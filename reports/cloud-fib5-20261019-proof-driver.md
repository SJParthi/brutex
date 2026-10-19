# FIB-5 real-data proof driver (D-6392) — the two files

The read-only scratch program of `reports/cloud-fib5-20261019.md`, NEEDS THE
MAC item 5. It is NOT part of the workspace: write these two files into a new
scratch directory outside the worktree, replace `WORKTREE` in `Cargo.toml`
with the merged worktree's absolute path, and follow the report's steps.

Both files are byte for byte the ones compile-checked and clippy-checked on
19 Oct 2026 against a local, never-pushed merge of `cloud/fib5-20261019`
with `origin/cloud/fib8-20261018` at `ceae9644` (with a local placeholder for
FIB-5's `SlotOutcome` fields in FIB-8's `files.rs`, the report's item 4). It
was never run: there is no store in the cloud.

## `Cargo.toml`

```toml
[package]
name = "fib5-proof"
version = "0.0.0"
edition = "2024"
publish = false
description = "FIB-5's read-only real-data proof: a scratch program with path dependencies, run on the Mac once FIB-8 is merged (D-6392)"

# Its own workspace: never a member of the brutex workspace.
[workspace]

[dependencies]
# WORKTREE is the absolute path of the merged brutex worktree (FIB-5 and
# FIB-8 merged into feat/db-fix).
cli = { path = "WORKTREE/crates/cli" }
runner = { path = "WORKTREE/crates/runner" }
indicators = { path = "WORKTREE/crates/indicators" }
vocab = { path = "WORKTREE/crates/vocab" }
pull = { path = "WORKTREE/crates/pull" }
```

## `src/main.rs`

```rust
//! FIB-5's real-data proof (D-6392): a READ-ONLY scratch program with path
//! dependencies, run on the Mac once FIB-5 and FIB-8 are merged.
//!
//! It reads stock-months through FIB-8's day input (Zerodha one-minute bars
//! through `cli::stored`'s door, folded in the run from 09:15 by `pull::fold`;
//! GDFL seconds of the continuous phase), plans a CLOSE trade at the close of
//! every usable candle of the given width on both sides with the stop beyond
//! that candle's far edge (`runner::fib::walk::proof::plans_at_every_close`),
//! walks them at stage A on the minutes and in deep on the seconds, and
//! tallies what the FIB-5 specification asks (`runner::fib::walk::proof::Tally`).
//! It writes one text file into a NEW directory and nothing else.
//!
//! Every result is labelled: equities are research, gross of every charge;
//! deep runs with the factor pair (1, 1) — "factor 1 assumed, development only,
//! not a result" — the clock lag unchecked and no book evidence.
//!
//! ```text
//! fib5-proof STORE_ROOT NEW_OUT_DIR WIDTH_MINUTES TASK...
//!   TASK = walk:SYMBOL:YYYY-MM:minute | walk:SYMBOL:YYYY-MM:minute+second
//!        | window:SYMBOL:YYYY-MM
//! ```

use std::fmt::Write as _;
use std::path::Path;

use cli::fib::authorities::{Authorities, SeriesId};
use cli::fib::day::{
    MonthSpec, PreviousReference, SecondsDay, StockState, build_month, load_seconds,
    load_series_month,
};
use indicators::Candle;
use indicators::fibflag::types::FibCandle;
use runner::fib::types::{
    BeyondTarget, ClockRule, CrossFeed, ExitFill, HaltRule, PriceMap, Resolution, SnapshotEdge,
    TradeSize, WalkProfile,
};
use runner::fib::walk::exits::{ExitSet, ExitVariant, FillVariant, PassBudget, StopLoss};
use runner::fib::walk::proof::{Tally, plans_at_every_close};
use runner::fib::walk::{
    BookEvidence, ClockLag, DayInput, HaltRange, Plan, TrailCandles, index_day, walk_all,
};
use vocab::fib_cube::{SESSION_AUTO_EXIT_BY_SECS, SESSION_FORCED_WINDOW_START_SECS, Search};

/// Whether second-of-day `s` lies in the forced window `[15:09:00, 15:10:00)`.
fn in_window(s: u32) -> bool {
    (SESSION_FORCED_WINDOW_START_SECS..SESSION_AUTO_EXIT_BY_SECS).contains(&u64::from(s))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match run(&args) {
        Ok(()) => 0,
        Err(why) => {
            eprintln!("fib5-proof: {why}");
            1
        }
    };
    std::process::exit(code);
}

fn run(args: &[String]) -> Result<(), String> {
    let [_, root, out, width, tasks @ ..] = args else {
        return Err("usage: fib5-proof STORE_ROOT NEW_OUT_DIR WIDTH_MINUTES TASK...".into());
    };
    let root = Path::new(root);
    let width: u16 = width.parse().map_err(|_| format!("width `{width}`"))?;
    if tasks.is_empty() {
        return Err("no task".into());
    }
    // A NEW directory: an existing one is refused, never written into.
    std::fs::create_dir(out).map_err(|why| format!("{out}: {why}"))?;
    let authorities = Authorities::interim(&|_| true)?;
    let mut report = String::new();
    let _ = writeln!(
        report,
        "FIB-5 real-data proof (D-6392). Equities are research, gross of every charge. \
         Deep: factor 1 assumed, development only, not a result; clock lag unchecked; \
         no book evidence. Width {width} minutes. Store {}.",
        root.display()
    );
    for task in tasks {
        let parts: Vec<&str> = task.split(':').collect();
        match parts.as_slice() {
            ["walk", symbol, month, stages] => {
                walk_month(
                    root,
                    &authorities,
                    symbol,
                    month,
                    stages,
                    width,
                    &mut report,
                )?;
            }
            ["window", symbol, month] => {
                window_month(root, &authorities, symbol, month, &mut report)?
            }
            _ => return Err(format!("task `{task}`")),
        }
    }
    let path = Path::new(out).join("fib5-proof.txt");
    std::fs::write(&path, report).map_err(|why| format!("{}: {why}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn year_month(word: &str) -> Result<(u16, u8), String> {
    let (year, month) = word.split_once('-').ok_or(format!("month `{word}`"))?;
    Ok((
        year.parse().map_err(|_| format!("year `{year}`"))?,
        month.parse().map_err(|_| format!("month `{month}`"))?,
    ))
}

/// One stock-month of FIB-8's day input, its cash closes and its series.
struct Month {
    input: cli::fib::day::MonthInput,
    minutes: Vec<Candle>,
    cash: cli::stored::CashCloses,
}

fn load(
    root: &Path,
    authorities: &Authorities,
    symbol: &str,
    when: (u16, u8),
    widths: &[u16],
) -> Result<Month, String> {
    let first = pull::session::Day::new(when.0, when.1, 1)
        .map(|day| i64::from(day.days_from_epoch()))
        .map_err(|why| format!("{why}"))?;
    let series: SeriesId = authorities
        .pool
        .member_named(symbol, first)
        .map(|member| member.series_id)
        .ok_or(format!("not_in_pool: {symbol}"))?;
    let current = load_series_month(root, authorities.pool.as_ref(), series, when)
        .map_err(|halt| halt.to_string())?;
    let days: Vec<i64> = current
        .minutes
        .iter()
        .map(|bar| cli::fib::day::ist_day_second(bar.ts_micros).0)
        .collect();
    let cash = cli::stored::load_cash_closes(root, symbol, days)?;
    let spec = MonthSpec {
        symbol,
        series,
        year: when.0,
        month: when.1,
        widths,
        reference: PreviousReference::TradedSession,
        daily: None,
        rename_conflicts: &current.conflicts,
    };
    let mut state = StockState::first_day();
    let input = build_month(&spec, &current.minutes, &cash, authorities, &mut state)?;
    Ok(Month {
        input,
        minutes: current.minutes,
        cash,
    })
}

/// The greatest common divisor of every positive price of `bars`: the
/// observed tick of their feed, or `None` for no bar.
fn observed_tick<'a>(bars: impl IntoIterator<Item = &'a Candle>) -> Option<i64> {
    let gcd = |mut a: i64, mut b: i64| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    bars.into_iter()
        .flat_map(|c| [c.open, c.high, c.low, c.close])
        .filter(|p| *p > 0)
        .reduce(gcd)
}

/// The day's seconds, ascending.
fn seconds_of(day: &SecondsDay) -> Vec<Candle> {
    (0..86_400).filter_map(|s| day.at(s).copied()).collect()
}

/// The longest run of missing seconds between two kept seconds.
fn longest_gap(seconds: &[Candle]) -> u32 {
    seconds
        .windows(2)
        .map(|w| {
            let a = cli::fib::day::ist_day_second(w[0].ts_micros).1;
            let b = cli::fib::day::ist_day_second(w[1].ts_micros).1;
            b.saturating_sub(a).saturating_sub(1)
        })
        .max()
        .unwrap_or(0)
}

/// One tally's key: direction, exit variant, size, fill reading.
type Key = (Search, &'static str, &'static str, &'static str);

/// The tallies of one stage, by key.
type Tallies = Vec<(Key, Tally)>;

fn tally_of(tallies: &mut Tallies, key: Key) -> &mut Tally {
    if let Some(at) = tallies.iter().position(|(k, _)| *k == key) {
        return &mut tallies[at].1;
    }
    tallies.push((key, Tally::empty()));
    let last = tallies.len() - 1;
    &mut tallies[last].1
}

/// The proof's exit set on `direction`: the reference walk and the stop
/// beyond the base, no other component.
fn exits(direction: Search) -> Result<ExitSet, String> {
    let stop = ExitVariant {
        stop: StopLoss::BeyondBase { extra_ticks: 0 },
        ..ExitVariant::REFERENCE
    };
    ExitSet::new(direction, 1, 0, vec![ExitVariant::REFERENCE, stop])
        .map_err(|why| why.name().to_owned())
}

const VARIANTS: [&str; 2] = ["reference", "stop_beyond_base"];
const SIZES: [TradeSize; 2] = [TradeSize::Small, TradeSize::Large];

#[allow(clippy::too_many_arguments)]
fn walk_month(
    root: &Path,
    authorities: &Authorities,
    symbol: &str,
    month: &str,
    stages: &str,
    width: u16,
    report: &mut String,
) -> Result<(), String> {
    let when = year_month(month)?;
    let deep = match stages {
        "minute" => false,
        "minute+second" => true,
        _ => return Err(format!("stages `{stages}`")),
    };
    let widths = [1, width];
    let loaded = load(root, authorities, symbol, when, &widths)?;
    let minute_days: Vec<i64> = loaded.input.days.iter().map(|d| d.day).collect();
    let seconds = if deep {
        Some(
            load_seconds(root, symbol, when, &minute_days, &loaded.cash)
                .map_err(|halt| halt.to_string())?,
        )
    } else {
        None
    };
    let mut stage_a: Tallies = Vec::new();
    let mut stage_deep: Tallies = Vec::new();
    let mut walked_days = 0_u32;
    let mut window_empty = 0_u32;
    let mut past_minutes: Vec<Candle> = Vec::new();
    let mut past_seconds: Vec<Candle> = Vec::new();
    let mut past_gap: Option<u32> = None;
    let _ = writeln!(report, "\n== {symbol} {month} ({stages})");
    for day in &loaded.input.days {
        let widths_of = if day.candles.iter().any(|w| !w.candles.is_empty()) {
            &day.candles
        } else if let Some(exposure) = &day.exposure {
            &exposure.candles
        } else {
            let _ = writeln!(
                report,
                "day {}: no candles (refused: {:?})",
                day.day,
                day.refusal.map(|r| r.name())
            );
            continue;
        };
        let one: Vec<FibCandle> = widths_of
            .iter()
            .filter(|w| w.width == 1)
            .flat_map(|w| w.candles.clone())
            .collect();
        let decision: Vec<FibCandle> = widths_of
            .iter()
            .filter(|w| w.width == width)
            .flat_map(|w| w.candles.clone())
            .collect();
        let minutes: Vec<Candle> = one
            .iter()
            .filter(|c| c.complete)
            .map(|c| c.candle)
            .collect();
        let halts: Vec<HaltRange> = pull::calendar::halt_on(day.day)
            .map(|halt| HaltRange {
                start_second: halt.cash.from,
                end_second: halt.cash.resume,
            })
            .into_iter()
            .collect();
        let trail = [TrailCandles {
            tf_minutes: width,
            candles: &decision,
        }];
        let decision_tick = observed_tick(past_minutes.iter())
            .or_else(|| observed_tick(minutes.iter()))
            .ok_or("no tick")?;
        walked_days += 1;
        let plans: Vec<(Search, Vec<Plan>)> = [Search::Long, Search::Short]
            .into_iter()
            .map(|direction| (direction, plans_at_every_close(&decision, 0, direction)))
            .collect();
        // Stage A: Zerodha minutes, identity, whole-quantity fills.
        let profile_a = WalkProfile {
            resolution: Resolution::Minute,
            price_map: PriceMap::Identity,
            exit_fill: ExitFill::SpillByVolume,
            snapshot_edge: SnapshotEdge::StampOpensInterval,
            halt_rule: HaltRule::WorstSeenThroughHalt,
            cross_feed: CrossFeed::SecondsOnly,
            beyond_target: BeyondTarget::FiresOnFirstBar,
            clock_rule: ClockRule::RefuseShifted,
        };
        let input_a = DayInput {
            bars: &minutes,
            book: BookEvidence::NotObserved,
            halts: &halts,
            zerodha_minutes: &[],
            raw_tick: decision_tick,
            decision_tick,
            max_second_gap: u32::MAX,
            clock_lag: ClockLag::Measured(0),
            trail: &trail,
        };
        walk_stage(
            &profile_a,
            &input_a,
            &plans,
            FillVariant::STAGE_A,
            &mut stage_a,
            report,
            day.day,
        )?;
        if let Some(seconds) = &seconds {
            let Some(held) = seconds.day(day.day) else {
                window_empty += 1;
                let _ = writeln!(report, "day {}: no execution seconds", day.day);
                past_minutes.extend(minutes);
                continue;
            };
            let bars = seconds_of(held);
            if !bars
                .iter()
                .any(|b| in_window(cli::fib::day::ist_day_second(b.ts_micros).1))
            {
                window_empty += 1;
            }
            let raw_tick = observed_tick(past_seconds.iter())
                .or_else(|| observed_tick(bars.iter()))
                .ok_or("no raw tick")?;
            let profile_d = WalkProfile {
                resolution: Resolution::Second,
                ..profile_a
            };
            let input_d = DayInput {
                bars: &bars,
                book: BookEvidence::NotObserved,
                halts: &halts,
                zerodha_minutes: &minutes,
                raw_tick,
                decision_tick,
                // The committed past's longest silence; none before the first
                // day walked, so no silence is flagged on it (labelled).
                max_second_gap: past_gap.unwrap_or(u32::MAX),
                clock_lag: ClockLag::TrustUnchecked,
                trail: &trail,
            };
            for fill in [FillVariant::STAGE_A, FillVariant::REFERENCE] {
                walk_stage(
                    &profile_d,
                    &input_d,
                    &plans,
                    fill,
                    &mut stage_deep,
                    report,
                    day.day,
                )?;
            }
            let gap = longest_gap(&bars);
            past_gap = Some(past_gap.map_or(gap, |g| g.max(gap)));
            past_seconds.extend(bars);
        }
        past_minutes.extend(minutes);
    }
    let _ = writeln!(
        report,
        "days walked: {walked_days}; first day's tick and gap read from that day itself, every later day's from the days before it"
    );
    print_tallies("stage A (Zerodha minutes)", &stage_a, report);
    if deep {
        print_tallies(
            "deep (GDFL seconds, factor 1 assumed, development only, not a result)",
            &stage_deep,
            report,
        );
        let _ = writeln!(
            report,
            "days with no GDFL second in [15:09:00, 15:10:00): {window_empty} of {walked_days}"
        );
    }
    Ok(())
}

fn walk_stage(
    profile: &WalkProfile,
    input: &DayInput<'_>,
    plans: &[(Search, Vec<Plan>)],
    fill: FillVariant,
    tallies: &mut Tallies,
    report: &mut String,
    day: i64,
) -> Result<(), String> {
    for (direction, plans) in plans {
        let set = exits(*direction)?;
        let index = match index_day(profile, input, &set) {
            Ok(index) => index,
            Err(why) => {
                let _ = writeln!(report, "day {day}: index refused {}", why.name());
                return Ok(());
            }
        };
        let budget = PassBudget::new(set.count()).ok_or("budget")?;
        let walked = walk_all(&index, plans, &set, fill, &SIZES, budget)
            .map_err(|why| why.name().to_owned())?;
        let reading = match (profile.resolution, fill == FillVariant::STAGE_A) {
            (Resolution::Minute, _) => "stage A",
            (Resolution::Second, true) => "reading 1 (next close)",
            (Resolution::Second, false) => "reading 2 (until 15:09:00)",
        };
        for pair in walked {
            let variant = VARIANTS[usize::from(pair.variant)];
            for (size, outcome) in SIZES.iter().zip(&pair.slots) {
                let size = match size {
                    TradeSize::Small => "Rs 1,00,000",
                    TradeSize::Large => "Rs 5,00,000",
                };
                tally_of(tallies, (*direction, variant, size, reading)).add(*direction, outcome);
            }
        }
    }
    Ok(())
}

fn print_tallies(stage: &str, tallies: &Tallies, report: &mut String) {
    let _ = writeln!(report, "-- {stage}");
    for ((direction, variant, size, reading), tally) in tallies {
        let mut waits = tally.waits.clone();
        waits.sort_unstable();
        let p99 = if waits.is_empty() {
            None
        } else {
            let rank = (99 * waits.len()).div_ceil(100);
            waits.get(rank.saturating_sub(1)).copied()
        };
        let worst = tally.worst_r.map(|r| format!("{}/{}", r.num, r.den));
        let _ = writeln!(
            report,
            "{direction:?} {variant} {size} {reading}: slots {}; statuses {:?}; exit reasons (priced) {:?}; \
             gap_through_stop {}; worst R {}; r_incomparable {}; partial entries {}; \
             square-offs {}; seconds waited p99 {} over {} entered; longest spill {}",
            tally.slots,
            tally.statuses,
            tally.reasons,
            tally.gap_through_stop,
            worst.unwrap_or_else(|| "none".into()),
            tally.r_incomparable,
            tally.partial_entries,
            tally.square_offs,
            p99.map_or_else(|| "none".into(), |p| p.to_string()),
            tally.waits.len(),
            tally.spills.iter().max().copied().unwrap_or(0),
        );
    }
}

/// The share of a stock-month's days with Zerodha minutes and no GDFL second
/// in the forced window.
fn window_month(
    root: &Path,
    authorities: &Authorities,
    symbol: &str,
    month: &str,
    report: &mut String,
) -> Result<(), String> {
    let when = year_month(month)?;
    let loaded = load(root, authorities, symbol, when, &[1])?;
    let minute_days: Vec<i64> = loaded.input.days.iter().map(|d| d.day).collect();
    let seconds = load_seconds(root, symbol, when, &minute_days, &loaded.cash)
        .map_err(|halt| halt.to_string())?;
    let empty = minute_days
        .iter()
        .filter(|day| {
            seconds.day(**day).is_none_or(|held| {
                (0..86_400)
                    .filter(|s| in_window(*s))
                    .all(|s| held.at(s).is_none())
            })
        })
        .count();
    let _ = writeln!(
        report,
        "window {symbol} {month}: {empty} of {} stock-days with minutes have no GDFL second in [15:09:00, 15:10:00) ({} minutes read)",
        minute_days.len(),
        loaded.minutes.len()
    );
    Ok(())
}
```
