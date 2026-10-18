//! FIB-6's scoring pass, proved against an independent reference scorer and
//! hand-built sequences (D-6402, D-6403, D-6409, D-6410). Every fixture is
//! synthetic: round invented paisa, days from an arbitrary origin.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    clippy::float_cmp,
    clippy::type_complexity,
    clippy::items_after_statements,
    clippy::collapsible_if,
    clippy::cast_possible_wrap,
    reason = "a test that cannot panic cannot fail; statistics are compared bit for bit; the reference scorer is written plainly"
)]

#[path = "fib6/fixture.rs"]
mod fixture;

use std::collections::{BTreeSet, HashMap};

use fixture::{Seeded, base_cell, decision, generation, priced};
use runner::fib::lattice::{Decision, Partition, RowTable};
use runner::fib::score::{
    Calendar, Card, ChargeModel, DayKeys, Drawdown, Gross, MonthStats, Order, OrderedPass, Outcome,
    PERIOD_KINDS, PassCounts, PeriodKind, PeriodStats, Profile, Rungs, SCORECARD_LABEL,
    SIDE_VALUES, SIZE_COUNT, SIZES, ScoreRefusal, SideStats, Slot, Stat, Streaks, Unpriced,
    profiles_of, size_index,
};
use runner::fib::types::{TradeFlag, TradeFlags, TradeSize};
use vocab::fib_cube::{BP_PER_UNIT, Ratio, RowId, Search};

/// One decision of a hand-built case: stock, day, decision second, side value,
/// and its slot at the small size (the large size is the same slot).
struct Entry {
    slot: u32,
    day: u32,
    second: u32,
    side: u8,
    outcome: Slot,
}

/// The table and outcomes of a hand-built case of one exit variant, months
/// of 30 days from day 100.
fn case(search: Search, entries: &[Entry]) -> (RowTable, Vec<Outcome>) {
    let generation = generation(search, 2, 2, 1);
    let mut months: HashMap<u32, Vec<(Decision, Vec<u32>)>> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        let mut cell = base_cell(search);
        cell[RowId::Side.index()] = e.side;
        months.entry((e.day - 100) / 30).or_default().push((
            decision(cell, e.slot, e.day, e.second, i as u64),
            vec![i as u32],
        ));
    }
    let mut data: Vec<_> = months.into_iter().collect();
    data.sort_by_key(|(m, _)| *m);
    let table = fixture::table_of(&generation, fixture::partition(search, 1), 4, data);
    let outcomes = entries
        .iter()
        .map(|e| Outcome {
            slots: [e.outcome; SIZE_COUNT],
        })
        .collect();
    (table, outcomes)
}

/// Every row of a table, ascending.
fn all_rows(table: &RowTable) -> Vec<u32> {
    (0..table.len() as u32).collect()
}

/// Scores every row of a table for its small-size profiles with no charges.
fn score_small(table: &RowTable, outcomes: &[Outcome]) -> Card {
    let profiles: Vec<Profile> = profiles_of(table)
        .into_iter()
        .filter(|p| p.size == TradeSize::Small)
        .collect();
    let mut pass =
        OrderedPass::new(table, &profiles, no_rungs(), vec![Gross; profiles.len()]).unwrap();
    let calendar = fixture::calendar(100, 120);
    pass.score(table, &all_rows(table), outcomes, &calendar)
        .unwrap()[0]
        .clone()
}

/// Empty ladders.
fn no_rungs() -> Rungs {
    Rungs::new(Vec::new(), Vec::new()).unwrap()
}

/// A long trade of 10 shares at Rs 1,000.00 entry (synthetic), risk 100 paisa
/// a share, gaining `gain` paisa a share, exiting at `exit`.
fn trade(gain: i128, entry_second: u32, exit: u32) -> Slot {
    Slot::Priced(priced(
        10,
        1_000_000,
        1_000_000 + 10 * gain,
        100,
        entry_second,
        exit,
    ))
}

/// The day's seconds for a clock time (synthetic: hours and minutes IST).
const fn at(hour: u32, minute: u32) -> u32 {
    hour * 3_600 + minute * 60
}

/// D-6402: overlapping trades on one stock — the later candidate is skipped
/// while the position is open, also exactly at its exit second, and taken one
/// second after.
#[test]
fn one_position_per_stock_skips_overlapping_trades() {
    let entries = [
        Entry {
            slot: 3,
            day: 100,
            second: at(10, 0),
            side: 0,
            outcome: trade(50, at(10, 0), at(11, 0)),
        },
        Entry {
            slot: 3,
            day: 100,
            second: at(10, 30),
            side: 0,
            outcome: trade(50, at(10, 30), at(10, 45)),
        },
        Entry {
            slot: 3,
            day: 100,
            second: at(11, 0),
            side: 0,
            outcome: trade(50, at(11, 0), at(11, 30)),
        },
        Entry {
            slot: 3,
            day: 100,
            second: at(11, 0) + 1,
            side: 0,
            outcome: trade(-20, at(11, 1), at(12, 0)),
        },
        Entry {
            slot: 4,
            day: 100,
            second: at(10, 30),
            side: 0,
            outcome: trade(50, at(10, 30), at(10, 45)),
        },
    ];
    let (table, outcomes) = case(Search::Long, &entries);
    let card = score_small(&table, &outcomes);
    assert_eq!(card.counts.raw, 5);
    assert_eq!(
        card.counts.trades, 3,
        "10:00 and 11:00:01 on stock 3, 10:30 on stock 4"
    );
    assert_eq!(card.counts.blocked_while_open, 2);
    assert_eq!(card.counts.ties, 0);
    assert_eq!(card.months[0].wins, 2);
    assert_eq!(card.months[0].losses, 1);
    assert!(
        card.counts.raw >= card.counts.trades,
        "the raw count bounds the effective count"
    );
}

/// Two signals on one stock in one second: one trade, one tie counted; 50
/// stocks in one second: 50 trades.
#[test]
fn two_signals_on_one_stock_in_one_second_and_fifty_stocks_in_one_second() {
    let mut entries = vec![
        Entry {
            slot: 1,
            day: 101,
            second: at(9, 30),
            side: 0,
            outcome: trade(10, at(9, 30), at(9, 40)),
        },
        Entry {
            slot: 1,
            day: 101,
            second: at(9, 30),
            side: 0,
            outcome: trade(10, at(9, 30), at(9, 35)),
        },
    ];
    let (table, outcomes) = case(Search::Long, &entries);
    let card = score_small(&table, &outcomes);
    assert_eq!(
        (
            card.counts.trades,
            card.counts.blocked_while_open,
            card.counts.ties
        ),
        (1, 1, 1)
    );

    entries.clear();
    for slot in 0..50 {
        entries.push(Entry {
            slot,
            day: 102,
            second: at(10, 15),
            side: 0,
            outcome: trade(10, at(10, 15), at(10, 20)),
        });
    }
    let (table, outcomes) = case(Search::Long, &entries);
    let card = score_small(&table, &outcomes);
    assert_eq!(card.counts.trades, 50);
    assert_eq!(card.counts.blocked_while_open, 0);
    assert_eq!(card.months[0].trades, 50);
}

/// FM-F056: an unpriceable 10:00 long blocks a 13:00 long on the same stock
/// to the end of the day, and not the next day.
#[test]
fn an_unpriceable_long_blocks_a_later_long_to_the_end_of_the_day() {
    let entries = [
        Entry {
            slot: 7,
            day: 103,
            second: at(10, 0),
            side: 0,
            outcome: Slot::Unpriced(Unpriced::ForcedExitUnpriceable),
        },
        Entry {
            slot: 7,
            day: 103,
            second: at(13, 0),
            side: 0,
            outcome: trade(10, at(13, 0), at(14, 0)),
        },
        Entry {
            slot: 7,
            day: 104,
            second: at(9, 20),
            side: 0,
            outcome: trade(10, at(9, 20), at(9, 30)),
        },
    ];
    let (table, outcomes) = case(Search::Long, &entries);
    let card = score_small(&table, &outcomes);
    assert_eq!(card.counts.unpriced, 1);
    assert_eq!(card.counts.blocked_by_unpriced, 1);
    assert_eq!(card.counts.trades, 1, "the next day's trade");
    assert_eq!(card.months[0].unpriced, 1);
    assert_eq!(
        card.months[0].sum_r, 0.1,
        "only the next day's trade: the unpriced row adds no R"
    );
}

/// FM-F056: two BREAKs armed at one close give one trade (the first fills,
/// the second is blocked and counted as a tie); a BREAK that never fills
/// blocks to its window end; zero-quantity and inapplicable slots block
/// nothing. The tie count is golden.
#[test]
fn two_breaks_armed_at_one_close_give_one_trade_and_the_tie_count_is_golden() {
    let close = at(11, 15);
    let entries = [
        Entry {
            slot: 2,
            day: 105,
            second: close,
            side: 0,
            outcome: trade(30, close + 60, at(12, 0)),
        },
        Entry {
            slot: 2,
            day: 105,
            second: close,
            side: 0,
            outcome: Slot::Pending {
                until_second: at(11, 45),
            },
        },
        Entry {
            slot: 5,
            day: 105,
            second: close,
            side: 0,
            outcome: Slot::Pending {
                until_second: at(11, 45),
            },
        },
        Entry {
            slot: 5,
            day: 105,
            second: close,
            side: 0,
            outcome: trade(30, close + 60, at(12, 0)),
        },
        Entry {
            slot: 5,
            day: 105,
            second: at(11, 45),
            side: 0,
            outcome: trade(30, at(11, 45), at(12, 0)),
        },
        Entry {
            slot: 5,
            day: 105,
            second: at(11, 46),
            side: 0,
            outcome: Slot::ZeroQty,
        },
        Entry {
            slot: 5,
            day: 105,
            second: at(11, 47),
            side: 0,
            outcome: Slot::Inapplicable,
        },
        Entry {
            slot: 5,
            day: 105,
            second: at(11, 48),
            side: 0,
            outcome: trade(-30, at(11, 48), at(12, 30)),
        },
    ];
    let (table, outcomes) = case(Search::Long, &entries);
    let card = score_small(&table, &outcomes);
    let c = card.counts;
    assert_eq!(c.trades, 2, "stock 2's first BREAK, stock 5's 11:48 trade");
    assert_eq!(c.blocked_while_open, 1);
    assert_eq!(
        c.blocked_by_pending, 2,
        "the tie at the close and 11:45 exactly"
    );
    assert_eq!(c.ties, 2);
    assert_eq!(c.unfilled, 1);
    assert_eq!((c.zero_qty, c.inapplicable), (1, 1));
    assert_eq!(
        c.trades
            + c.blocked_while_open
            + c.blocked_by_pending
            + c.blocked_by_unpriced
            + c.unfilled
            + c.unpriced
            + c.zero_qty
            + c.inapplicable,
        c.raw,
        "every raw row is accounted for"
    );
}

/// A charge model of a fixed charge per order and a rounding adjustment per
/// closed day (synthetic rates, in units of 1/100 paisa).
#[derive(Clone, Debug)]
struct Flat {
    per_order: i128,
    per_day: i128,
    orders: u64,
}

impl ChargeModel for Flat {
    fn scale(&self) -> i128 {
        100
    }

    fn add(&mut self, order: &Order) -> i128 {
        assert!(order.qty > 0);
        self.orders += 1;
        self.per_order + i128::from(order.broker_square_off) * self.per_order
    }

    fn close_day(&mut self, _day: u32) -> i128 {
        self.per_day
    }
}

/// Net, never gross (operator 11:13 IST): a small gross win that charges turn
/// into a net loss counts as a LOSS; a net of exactly zero is a loss; the
/// day's rounding adjustment lands in its month's money; a broker square-off
/// order pays its extra charge.
#[test]
fn a_small_gross_win_that_charges_turn_into_a_net_loss_counts_as_a_loss() {
    let mut square = priced(
        10,
        1_000_000,
        1_000_000 + 10 * 60,
        100,
        at(10, 0),
        at(15, 9),
    );
    square.flags = TradeFlags::NONE.with(TradeFlag::DhanSquareOff);
    let entries = [
        Entry {
            slot: 1,
            day: 100,
            second: at(10, 0),
            side: 0,
            outcome: trade(1, at(10, 0), at(10, 5)),
        },
        Entry {
            slot: 2,
            day: 100,
            second: at(10, 0),
            side: 0,
            outcome: trade(4, at(10, 0), at(10, 5)),
        },
        Entry {
            slot: 3,
            day: 100,
            second: at(10, 0),
            side: 0,
            outcome: Slot::Priced(square),
        },
    ];
    let (table, outcomes) = case(Search::Long, &entries);
    let profiles = vec![Profile {
        variant: 0,
        size: TradeSize::Small,
    }];
    let model = Flat {
        per_order: 2_000,
        per_day: 7,
        orders: 0,
    };
    let mut pass = OrderedPass::new(&table, &profiles, no_rungs(), vec![model]).unwrap();
    let calendar = fixture::calendar(100, 30);
    let card = pass
        .score(&table, &all_rows(&table), &outcomes, &calendar)
        .unwrap()[0]
        .clone();
    let month = card.months[0];
    // Gross 10 paisa = 1,000 units, charges 4,000: net -3,000, a loss.
    // Gross 40 paisa = 4,000 units, charges 4,000: net 0, a loss.
    // Gross 600 paisa = 60,000 units, charges 2,000 + 4,000: net 54,000, a win.
    assert_eq!((month.wins, month.losses), (1, 2));
    assert_eq!(month.loss_money, -3_000);
    assert_eq!(month.win_money, 54_000);
    assert_eq!(month.adjustment_money, 7);
    assert_eq!(
        month.sum_r,
        -3_000.0 / 100_000.0 + 0.0 + 54_000.0 / 100_000.0
    );
    let gross = score_small(&table, &outcomes);
    assert_eq!(
        (gross.months[0].wins, gross.months[0].losses),
        (3, 0),
        "gross is only a labelled column"
    );
}

/// A reference scorer written the naive way: one hash map of open positions
/// per day, every figure recomputed from lists after the pass.
fn reference(
    table: &RowTable,
    rows: &[u32],
    outcomes: &[Outcome],
    calendar: &Calendar,
    profile: Profile,
    rungs: (&[Ratio], &[Ratio]),
) -> Card {
    let search = table.partition().search;
    let months = table.months() as usize;
    let mut card = Card {
        profile,
        months: vec![MonthStats::EMPTY; months],
        rung_net: vec![0; months * rungs.0.len()],
        rung_net_r: vec![0.0; months * rungs.0.len()],
        rung_mfe: vec![0; months * rungs.1.len()],
        counts: PassCounts::ZERO,
        streaks: Streaks::ZERO,
        drawdown: Drawdown::ZERO,
        periods: vec![PeriodStats::ZERO; PERIOD_KINDS],
    };
    let mut open: HashMap<u32, (u32, u32, u8)> = HashMap::new();
    let mut day_now = None;
    let mut closes: Vec<(u32, DayKeys, Stat)> = Vec::new();
    let mut day_r: Stat = 0.0;
    let mut day_traded = false;
    let mut results: Vec<(bool, Stat)> = Vec::new();
    let reached = |ladder: &[Ratio], num: i128, den: i128| {
        ladder
            .iter()
            .filter(|rung| num * i128::from(rung.den()) >= i128::from(rung.num()) * den)
            .count()
    };
    for row in rows {
        let view = table.row(*row).unwrap();
        if day_now != Some(view.day) {
            if let Some(day) = day_now {
                if day_traded {
                    closes.push((day, *calendar.keys(day).unwrap(), day_r));
                }
            }
            day_now = Some(view.day);
            open.clear();
            day_r = 0.0;
            day_traded = false;
        }
        let offset = usize::from(profile.variant - table.partition().exits.start);
        let slot = outcomes[view.outcomes[offset] as usize].slots[size_index(profile.size)];
        card.counts.raw += 1;
        if let Some((until, decided, by)) = open.get(&view.slot).copied() {
            if view.second <= until {
                match by {
                    0 => card.counts.blocked_while_open += 1,
                    1 => card.counts.blocked_by_pending += 1,
                    _ => card.counts.blocked_by_unpriced += 1,
                }
                if view.second == decided {
                    card.counts.ties += 1;
                }
                continue;
            }
        }
        let month = view.month as usize;
        match slot {
            Slot::Priced(o) => {
                open.insert(view.slot, (o.exit_second, view.second, 0));
                let gross = match search {
                    Search::Long => o.exit_value_paisa - o.entry_value_paisa,
                    Search::Short => o.entry_value_paisa - o.exit_value_paisa,
                };
                let den = i128::from(o.filled_qty) * i128::from(o.risk_per_share_paisa);
                let r = gross as Stat / den as Stat;
                let win = gross > 0;
                let m = &mut card.months[month];
                m.trades += 1;
                m.sum_r += r;
                m.sum_r_squared += r * r;
                m.max_r = Some(m.max_r.map_or(r, |x: Stat| x.max(r)));
                m.min_r = Some(m.min_r.map_or(r, |x: Stat| x.min(r)));
                if win {
                    m.wins += 1;
                    m.sum_win_r += r;
                    m.win_money += gross;
                } else {
                    m.losses += 1;
                    m.sum_loss_r += r;
                    m.loss_money += gross;
                }
                let side = &mut m.per_side[usize::from(view.coords[RowId::Side.index()])];
                side.trades += 1;
                side.sum_r += r;
                let stop = (i128::from(o.risk_per_share_paisa)
                    * i128::from(o.filled_qty)
                    * i128::from(BP_PER_UNIT)) as Stat
                    / o.entry_value_paisa as Stat;
                m.sum_stop_bp += stop;
                m.min_stop_bp = Some(m.min_stop_bp.map_or(stop, |x: Stat| x.min(stop)));
                let risk = i128::from(o.risk_per_share_paisa);
                m.sum_mae_r += o.mae_to_exit_paisa as Stat / risk as Stat;
                m.sum_mfe_r += i128::from(o.mfe_to_exit_paisa) as Stat / risk as Stat;
                card.counts.trades += 1;
                if win {
                    let net = reached(rungs.0, gross, den);
                    if net > 0 {
                        card.rung_net[month * rungs.0.len() + net - 1] += 1;
                        card.rung_net_r[month * rungs.0.len() + net - 1] += r;
                    }
                }
                let mfe = reached(rungs.1, i128::from(o.mfe_to_exit_paisa), risk);
                if mfe > 0 {
                    card.rung_mfe[month * rungs.1.len() + mfe - 1] += 1;
                }
                results.push((win, r));
                day_r += r;
                day_traded = true;
            }
            Slot::Pending { until_second } => {
                card.counts.unfilled += 1;
                open.insert(view.slot, (until_second, view.second, 1));
            }
            Slot::Unpriced(_) => {
                card.counts.unpriced += 1;
                card.months[month].unpriced += 1;
                open.insert(view.slot, (u32::MAX, view.second, 2));
            }
            Slot::ZeroQty => card.counts.zero_qty += 1,
            Slot::Inapplicable => card.counts.inapplicable += 1,
        }
    }
    if let Some(day) = day_now {
        if day_traded {
            closes.push((day, *calendar.keys(day).unwrap(), day_r));
        }
    }
    // Streaks, from the list of results.
    let s = &mut card.streaks;
    for (win, r) in &results {
        if *win {
            s.wins_now += 1;
            s.losses_now = 0;
            s.losing_r_now = 0.0;
        } else {
            s.losses_now += 1;
            s.losing_r_now += r;
            s.wins_now = 0;
        }
        s.longest_wins = s.longest_wins.max(s.wins_now);
        s.longest_losses = s.longest_losses.max(s.losses_now);
        s.deepest_losing_r = s.deepest_losing_r.min(s.losing_r_now);
    }
    // Drawdown, from the equity curve of day closes.
    let curve: Vec<(u32, u32, Stat)> = closes
        .iter()
        .scan(0.0, |equity: &mut Stat, (day, keys, r)| {
            *equity += r;
            Some((*day, keys.tradable_index, *equity))
        })
        .collect();
    let d = &mut card.drawdown;
    let mut high: Stat = 0.0;
    let mut i = 0;
    while i < curve.len() {
        let (_, _, equity) = curve[i];
        if equity >= high {
            high = equity;
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i;
        while end < curve.len() && curve[end].2 < high {
            end += 1;
        }
        let episode = &curve[start..end];
        let trough =
            episode.iter().enumerate().fold(
                0,
                |best, (j, point)| if point.2 < episode[best].2 { j } else { best },
            );
        let fall = high - episode[trough].2;
        let recovered = curve.get(end);
        if fall > d.max_drawdown_r {
            d.max_drawdown_r = fall;
            d.max_drawdown_start_day = Some(episode[0].0);
            d.max_drawdown_trough_day = Some(episode[trough].0);
            d.max_drawdown_recovery_day = recovered.map(|p| p.0);
            d.max_drawdown_recovery_days = recovered.map(|p| p.1 - episode[trough].1);
        }
        let length = match recovered {
            Some(point) => point.1 - episode[0].1,
            None => curve.last().unwrap().1 - episode[0].1 + 1,
        };
        d.longest_underwater_days = d.longest_underwater_days.max(length);
        if recovered.is_none() {
            d.underwater_now_days = length;
        }
        i = end;
    }
    if let Some(last) = curve.last() {
        d.equity_r = last.2;
    }
    d.high_r = high;
    // Periods, grouped by key.
    for (k, kind) in PeriodKind::ALL.iter().enumerate() {
        let key = |day: u32, keys: &DayKeys| match kind {
            PeriodKind::Day => day,
            PeriodKind::Week => keys.week,
            PeriodKind::Fortnight => keys.fortnight,
            PeriodKind::Month => keys.month,
            PeriodKind::Quarter => keys.quarter,
            PeriodKind::HalfYear => keys.half_year,
            PeriodKind::Year => keys.year,
        };
        let mut sums: Vec<Stat> = Vec::new();
        let mut last = None;
        for (day, keys, r) in &closes {
            let here = key(*day, keys);
            if last == Some(here) {
                *sums.last_mut().unwrap() += r;
            } else {
                sums.push(*r);
                last = Some(here);
            }
        }
        let table = &mut card.periods[k];
        table.periods = sums.len() as u64;
        table.winning = sums.iter().filter(|s| **s > 0.0).count() as u64;
        table.losing = sums.iter().filter(|s| **s < 0.0).count() as u64;
        table.flat = sums.iter().filter(|s| **s == 0.0).count() as u64;
        table.worst_r = sums.iter().copied().reduce(Stat::min);
        let mut sorted = sums.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        table.best_r = sorted.first().copied();
        table.second_best_r = sorted.get(1).copied();
    }
    card
}

/// A random table of one exit variant over several stocks, days and months,
/// with every kind of slot.
fn random_score_case(rng: &mut Seeded, rows: u32) -> (RowTable, Vec<Outcome>) {
    let generation = generation(Search::Short, 2, 2, 2);
    let mut months: Vec<(u32, Vec<(Decision, Vec<u32>)>)> =
        (0..3).map(|m| (m, Vec::new())).collect();
    let mut outcomes = Vec::new();
    for i in 0..rows {
        let mut cell = base_cell(Search::Short);
        if rng.below(2) == 0 {
            cell[RowId::Side.index()] = 2;
        }
        cell[RowId::Weekday.index()] = rng.below(3) as u8;
        let month = rng.below(3) as u32;
        let day = 100 + month * 30 + rng.below(29) as u32;
        let second = at(9, 15) + rng.below(6 * 3_600) as u32;
        months[month as usize].1.push((
            decision(cell, rng.below(6) as u32, day, second, u64::from(i)),
            vec![2 * i, 2 * i + 1],
        ));
        for _ in 0..2 {
            let slot = |rng: &mut Seeded| match rng.below(12) {
                0 => Slot::Unpriced(Unpriced::NoExecutionSeconds),
                1 => Slot::Pending {
                    until_second: second + rng.below(600) as u32,
                },
                2 => Slot::ZeroQty,
                3 => Slot::Inapplicable,
                _ => {
                    let qty = 1 + rng.below(200);
                    let entry = 50_000 + i128::from(rng.below(100_000) as u32);
                    let gain = i128::from(rng.below(600) as u32) - 250;
                    let mut o = priced(
                        qty,
                        entry * i128::from(qty),
                        (entry - gain) * i128::from(qty),
                        40 + rng.below(200) as i64,
                        second,
                        second + 1 + rng.below(3_600) as u32,
                    );
                    o.mfe_to_exit_paisa = rng.below(800) as i64;
                    o.mae_to_exit_paisa = -(rng.below(300) as i64);
                    Slot::Priced(o)
                }
            };
            outcomes.push(Outcome {
                slots: [slot(rng), slot(rng)],
            });
        }
    }
    (
        fixture::table_of(&generation, fixture::partition(Search::Short, 2), 3, months),
        outcomes,
    )
}

/// The bytes of a card.
fn bytes(card: &Card) -> Vec<u8> {
    let mut out = Vec::new();
    card.push_bytes(&mut out);
    out
}

/// The month vectors, counts, streaks, drawdown, period tables and ladders of
/// the ordered pass equal the reference scorer's, bit for bit, on random
/// mixes; and FM-F050: one pass reused across mixes (epoch-stamped scratch)
/// equals a fresh pass per mix.
#[test]
fn month_vectors_and_scorecards_equal_brute_force_and_epoch_scratch_equals_reset_scratch() {
    let mut rng = Seeded(0x5C0E_0006);
    let net = vec![
        Ratio::new(1, 2).unwrap(),
        Ratio::whole(1),
        Ratio::new(3, 2).unwrap(),
    ];
    let mfe = vec![Ratio::whole(1), Ratio::whole(3)];
    let calendar = fixture::calendar(100, 120);
    let mut compared = 0;
    for _ in 0..12 {
        let rows = 1 + rng.below(1_500) as u32;
        let (table, outcomes) = random_score_case(&mut rng, rows);
        let profiles = profiles_of(&table);
        assert_eq!(profiles.len(), 4, "two exits times two sizes (FM-F117)");
        let rungs = Rungs::new(net.clone(), mfe.clone()).unwrap();
        let mut reused = OrderedPass::new(
            &table,
            &profiles,
            rungs.clone(),
            vec![Gross; profiles.len()],
        )
        .unwrap();
        assert_eq!(reused.profiles().collect::<Vec<_>>(), profiles);
        for weekday in [None, Some(0), Some(1), Some(2), None] {
            let rows: Vec<u32> = all_rows(&table)
                .into_iter()
                .filter(|r| {
                    weekday
                        .is_none_or(|w| table.row(*r).unwrap().coords[RowId::Weekday.index()] == w)
                })
                .collect();
            let mut fresh = OrderedPass::new(
                &table,
                &profiles,
                rungs.clone(),
                vec![Gross; profiles.len()],
            )
            .unwrap();
            let fresh_cards = fresh
                .score(&table, &rows, &outcomes, &calendar)
                .unwrap()
                .to_vec();
            let cards = reused.score(&table, &rows, &outcomes, &calendar).unwrap();
            for (card, fresh_card) in cards.iter().zip(&fresh_cards) {
                assert_eq!(
                    bytes(card),
                    bytes(fresh_card),
                    "epoch scratch equals a reset"
                );
                let want = reference(
                    &table,
                    &rows,
                    &outcomes,
                    &calendar,
                    card.profile,
                    (&net, &mfe),
                );
                assert_eq!(card.months, want.months);
                assert_eq!(card.counts, want.counts);
                assert_eq!(card.streaks, want.streaks);
                assert_eq!(card.drawdown, want.drawdown);
                assert_eq!(card.periods, want.periods);
                assert_eq!(
                    (&card.rung_net, &card.rung_net_r, &card.rung_mfe),
                    (&want.rung_net, &want.rung_net_r, &want.rung_mfe)
                );
                assert_eq!(bytes(card), bytes(&want));
                compared += card.counts.trades;
            }
        }
    }
    assert!(compared > 5_000, "{compared} trades compared");
}

/// D-6403: streaks in entry order and the day-close drawdown with its
/// recovery, on a hand-built sequence.
#[test]
fn streaks_and_drawdown_recovery_on_hand_built_sequences() {
    // One trade a day on days 100..=105, R +2, -1, -2, +1, +3, -1, then two
    // losses and a win on day 106.
    let gains = [200, -100, -200, 100, 300, -100];
    let mut entries: Vec<Entry> = gains
        .iter()
        .enumerate()
        .map(|(d, g)| Entry {
            slot: 0,
            day: 100 + d as u32,
            second: at(10, 0),
            side: 0,
            outcome: trade(*g, at(10, 0), at(10, 30)),
        })
        .collect();
    for (i, g) in [-50, -50, 100].iter().enumerate() {
        entries.push(Entry {
            slot: 0,
            day: 106,
            second: at(11, 0) + 3_600 * i as u32,
            side: 0,
            outcome: trade(*g, at(11, 0), at(11, 0) + 3_600 * i as u32 + 60),
        });
    }
    let (table, outcomes) = case(Search::Long, &entries);
    let card = score_small(&table, &outcomes);
    // Day closes: 2, 1, -1, 0, 3, 2, 2 (day 106: -0.5 - 0.5 + 1.0 = 0).
    let d = card.drawdown;
    assert_eq!(d.equity_r, 2.0);
    assert_eq!(d.high_r, 3.0);
    assert_eq!(d.max_drawdown_r, 3.0, "from 2 down to -1");
    assert_eq!(d.max_drawdown_start_day, Some(101));
    assert_eq!(d.max_drawdown_trough_day, Some(102));
    assert_eq!(d.max_drawdown_recovery_day, Some(104));
    assert_eq!(d.max_drawdown_recovery_days, Some(2));
    assert_eq!(
        d.longest_underwater_days, 3,
        "days 101, 102, 103 under water"
    );
    assert_eq!(d.underwater_now_days, 2, "days 105 and 106, still open");
    let s = card.streaks;
    assert_eq!(
        (s.longest_wins, s.longest_losses),
        (2, 3),
        "day 105 and day 106's first two"
    );
    assert_eq!((s.wins_now, s.losses_now), (1, 0));
    assert_eq!(s.deepest_losing_r, -3.0, "-1 then -2");
    assert_eq!(s.losing_r_now, 0.0);
    let day = card.periods[0];
    assert_eq!(
        (day.periods, day.winning, day.losing, day.flat),
        (7, 3, 3, 1)
    );
    assert_eq!(
        (day.worst_r, day.best_r, day.second_best_r),
        (Some(-2.0), Some(3.0), Some(2.0))
    );
    let week = card.periods[1];
    assert_eq!((week.periods, week.best_r), (1, Some(2.0)));
    assert_eq!(PeriodKind::ALL.len(), PERIOD_KINDS);
    assert_eq!(
        PeriodKind::ALL.iter().map(|k| k.name()).collect::<Vec<_>>(),
        [
            "day",
            "week",
            "fortnight",
            "month",
            "quarter",
            "half_year",
            "year"
        ]
    );
    assert_eq!(
        SCORECARD_LABEL,
        "scorecard (R, day close, entry order) - information"
    );
}

/// A deepening second episode replaces the first as the deepest, and an
/// unrecovered deepest episode has no recovery.
#[test]
fn a_deeper_later_episode_is_the_deepest_and_may_not_recover() {
    let gains = [100, -100, 200, -300, -100];
    let entries: Vec<Entry> = gains
        .iter()
        .enumerate()
        .map(|(d, g)| Entry {
            slot: 0,
            day: 100 + d as u32,
            second: at(10, 0),
            side: 0,
            outcome: trade(*g, at(10, 0), at(10, 30)),
        })
        .collect();
    let (table, outcomes) = case(Search::Long, &entries);
    let d = score_small(&table, &outcomes).drawdown;
    assert_eq!(d.max_drawdown_r, 4.0);
    assert_eq!(d.max_drawdown_start_day, Some(103));
    assert_eq!(d.max_drawdown_trough_day, Some(104));
    assert_eq!(d.max_drawdown_recovery_day, None);
    assert_eq!(d.max_drawdown_recovery_days, None);
    assert_eq!(d.longest_underwater_days, 2);
    assert_eq!(d.underwater_now_days, 2);
}

/// Per side value, and a short's gross is its entry less its exit.
#[test]
fn a_short_is_measured_from_its_entry_and_each_side_value_is_counted() {
    let entries = [
        Entry {
            slot: 0,
            day: 100,
            second: at(10, 0),
            side: 1,
            outcome: trade(-100, at(10, 0), at(10, 5)),
        },
        Entry {
            slot: 1,
            day: 100,
            second: at(10, 0),
            side: 2,
            outcome: trade(50, at(10, 0), at(10, 5)),
        },
    ];
    let (table, outcomes) = case(Search::Short, &entries);
    let card = score_small(&table, &outcomes);
    let month = card.months[0];
    assert_eq!(
        month.per_side[1],
        SideStats {
            trades: 1,
            sum_r: 1.0
        }
    );
    assert_eq!(
        month.per_side[2],
        SideStats {
            trades: 1,
            sum_r: -0.5
        }
    );
    assert_eq!(month.per_side[0].trades + month.per_side[3].trades, 0);
    assert_eq!(month.per_side.len(), SIDE_VALUES);
    assert_eq!((month.max_r, month.min_r), (Some(1.0), Some(-0.5)));
    assert_eq!(
        month.min_stop_bp,
        Some(10.0),
        "100 paisa on Rs 1,000.00 is 10 bp"
    );
    assert_eq!(month.sum_mfe_r, 2.0);
    assert_eq!(month.sum_mae_r, -2.0);
}

/// The report ladders count each winning trade at its highest net-R rung and
/// each trade at its highest MFE rung, exactly.
#[test]
fn the_report_ladders_count_each_trade_at_its_highest_rung() {
    let entries: Vec<Entry> = [100, 99, 250, 300, -100]
        .iter()
        .enumerate()
        .map(|(i, g)| Entry {
            slot: i as u32,
            day: 100,
            second: at(10, 0),
            side: 0,
            outcome: trade(*g, at(10, 0), at(10, 5)),
        })
        .collect();
    let (table, outcomes) = case(Search::Long, &entries);
    let rungs = Rungs::new(
        vec![Ratio::whole(1), Ratio::whole(3)],
        vec![Ratio::whole(1)],
    )
    .unwrap();
    let profiles = [Profile {
        variant: 0,
        size: TradeSize::Large,
    }];
    let mut pass = OrderedPass::new(&table, &profiles, rungs, vec![Gross]).unwrap();
    let card = pass
        .score(
            &table,
            &all_rows(&table),
            &outcomes,
            &fixture::calendar(100, 30),
        )
        .unwrap()[0]
        .clone();
    assert_eq!(
        &card.rung_net[..2],
        &[2, 1],
        "1R and 2.5R at rung one, 3R at rung two; 0.99R at none"
    );
    assert_eq!(&card.rung_net_r[..2], &[3.5, 3.0]);
    assert_eq!(card.rung_mfe[0], 5, "every trade's MFE is exactly 1R");
    assert_eq!(card.profile.size, TradeSize::Large);
}

/// Every refusal of the pass, the calendar and the ladders is by name.
#[test]
fn the_pass_refuses_by_name() {
    let entries = [Entry {
        slot: 0,
        day: 100,
        second: at(10, 0),
        side: 0,
        outcome: trade(10, at(10, 0), at(10, 5)),
    }];
    let (table, outcomes) = case(Search::Long, &entries);
    let profiles = profiles_of(&table);
    assert_eq!(
        OrderedPass::new(&table, &[], no_rungs(), Vec::<Gross>::new()).unwrap_err(),
        ScoreRefusal::ProfilesEmpty
    );
    assert_eq!(
        OrderedPass::new(&table, &profiles, no_rungs(), vec![Gross]).unwrap_err(),
        ScoreRefusal::ChargesPerProfile {
            profiles: 2,
            charges: 1
        }
    );
    assert_eq!(
        OrderedPass::new(
            &table,
            &[Profile {
                variant: 1,
                size: TradeSize::Small
            }],
            no_rungs(),
            vec![Gross]
        )
        .unwrap_err(),
        ScoreRefusal::ProfileOutsideExits { variant: 1 }
    );
    let calendar = fixture::calendar(100, 30);
    let mut pass = OrderedPass::new(&table, &profiles, no_rungs(), vec![Gross; 2]).unwrap();
    assert_eq!(
        pass.score(&table, &[1], &outcomes, &calendar).unwrap_err(),
        ScoreRefusal::RowOutOfTable { row: 1 }
    );
    assert_eq!(
        pass.score(&table, &[0], &[], &calendar).unwrap_err(),
        ScoreRefusal::OutcomeOutOfRange {
            index: 0,
            outcomes: 0
        }
    );
    let late = fixture::calendar(101, 30);
    assert_eq!(
        pass.score(&table, &[0], &outcomes, &late).unwrap_err(),
        ScoreRefusal::DayNotTradable { day: 100 }
    );
    let holes = Calendar::new(100, vec![None]).unwrap();
    assert_eq!(holes.keys(100), None);
    assert_eq!(
        pass.score(&table, &[0], &outcomes, &holes).unwrap_err(),
        ScoreRefusal::DayNotTradable { day: 100 }
    );
    let mut bad = priced(0, 1, 1, 1, 0, 1);
    let refuse = |pass: &mut OrderedPass<Gross>, slot: Slot| {
        let outcomes = [Outcome {
            slots: [slot; SIZE_COUNT],
        }];
        pass.score(&table, &[0], &outcomes, &calendar).unwrap_err()
    };
    assert_eq!(
        refuse(&mut pass, Slot::Priced(bad)),
        ScoreRefusal::PricedWithoutQty { row: 0 }
    );
    bad.filled_qty = 1;
    bad.risk_per_share_paisa = 0;
    assert_eq!(
        refuse(&mut pass, Slot::Priced(bad)),
        ScoreRefusal::RiskNotPositive { row: 0 }
    );
    bad.risk_per_share_paisa = 1;
    bad.entry_value_paisa = 0;
    assert_eq!(
        refuse(&mut pass, Slot::Priced(bad)),
        ScoreRefusal::EntryValueNotPositive { row: 0 }
    );
    bad.entry_value_paisa = 1;
    bad.exit_value_paisa = i128::MIN;
    assert_eq!(
        refuse(&mut pass, Slot::Priced(bad)),
        ScoreRefusal::MoneyOverflow { row: 0 }
    );
    struct Huge;
    impl ChargeModel for Huge {
        fn scale(&self) -> i128 {
            i128::MAX
        }
        fn add(&mut self, _order: &Order) -> i128 {
            i128::MAX
        }
        fn close_day(&mut self, _day: u32) -> i128 {
            0
        }
    }
    let mut huge = OrderedPass::new(&table, &profiles[..1], no_rungs(), vec![Huge]).unwrap();
    let huge_outcomes = [Outcome {
        slots: [Slot::Priced(priced(1, 1, 2, 1, 0, 1)); SIZE_COUNT],
    }];
    assert_eq!(
        huge.score(&table, &[0], &huge_outcomes, &calendar)
            .unwrap_err(),
        ScoreRefusal::MoneyOverflow { row: 0 },
        "the two orders' charges overflow"
    );

    let other = case(
        Search::Short,
        &[Entry {
            slot: 0,
            day: 100,
            second: at(10, 0),
            side: 1,
            outcome: Slot::ZeroQty,
        }],
    )
    .0;
    assert_eq!(
        pass.score(&other, &[0], &outcomes, &calendar).unwrap_err(),
        ScoreRefusal::TableMismatch { what: "partition" }
    );
    let wide = case(
        Search::Long,
        &[Entry {
            slot: 9,
            day: 100,
            second: at(10, 0),
            side: 0,
            outcome: Slot::ZeroQty,
        }],
    )
    .0;
    assert_eq!(
        pass.score(&wide, &[0], &outcomes, &calendar).unwrap_err(),
        ScoreRefusal::TableMismatch { what: "pool" }
    );
    let generation = generation(Search::Long, 2, 2, 1);
    let longer =
        RowTable::new(*generation.layout(), fixture::partition(Search::Long, 1), 9).unwrap();
    assert_eq!(
        pass.score(&longer, &[], &outcomes, &calendar).unwrap_err(),
        ScoreRefusal::TableMismatch { what: "months" }
    );
    let card = pass.score(&table, &[0], &outcomes, &calendar).unwrap()[0].clone();
    assert_eq!(card.counts.trades, 1, "a refused mix leaves nothing behind");

    let keys = |index: u32, week: u32| DayKeys {
        tradable_index: index,
        week,
        fortnight: 0,
        month: 0,
        quarter: 0,
        half_year: 0,
        year: 0,
    };
    assert_eq!(
        Calendar::new(10, vec![Some(keys(3, 0)), None, Some(keys(3, 0))]).unwrap_err(),
        ScoreRefusal::CalendarIndexNotIncreasing { day: 12 }
    );
    assert_eq!(
        Calendar::new(10, vec![Some(keys(3, 2)), Some(keys(4, 1))]).unwrap_err(),
        ScoreRefusal::CalendarKeyDecreasing {
            day: 11,
            period: "week"
        }
    );
    assert_eq!(Calendar::new(10, Vec::new()).unwrap().keys(9), None);
    assert_eq!(
        Rungs::new(vec![Ratio::whole(2), Ratio::whole(2)], Vec::new()).unwrap_err(),
        ScoreRefusal::RungsNotIncreasing { ladder: "net_r" }
    );
    assert_eq!(
        Rungs::new(Vec::new(), vec![Ratio::whole(3), Ratio::whole(1)]).unwrap_err(),
        ScoreRefusal::RungsNotIncreasing { ladder: "mfe_r" }
    );
}

/// The profiles are the table's own exits times the two report sizes
/// (FM-F117), the sizes are in ledger order, and the names are stable.
#[test]
fn profiles_come_from_the_run_and_every_name_is_stable() {
    let generation = generation(Search::Long, 2, 2, 5);
    let part = Partition {
        exits: 1..4,
        ..fixture::partition(Search::Long, 5)
    };
    let table = RowTable::new(*generation.layout(), part, 1).unwrap();
    let profiles = profiles_of(&table);
    assert_eq!(profiles.len(), 6);
    assert_eq!(
        profiles[0],
        Profile {
            variant: 1,
            size: TradeSize::Small
        }
    );
    assert_eq!(
        profiles[5],
        Profile {
            variant: 3,
            size: TradeSize::Large
        }
    );
    assert_eq!(SIZES.len(), SIZE_COUNT);
    for (at, size) in SIZES.iter().enumerate() {
        assert_eq!(size_index(*size), at);
    }
    assert_eq!(SIZES[0].rupees(), 100_000);
    let reasons = [
        Unpriced::NoExecutionSeconds,
        Unpriced::FactorRefused,
        Unpriced::ForcedExitUnpriceable,
        Unpriced::FeedHole,
        Unpriced::CorruptBar,
    ];
    let names: BTreeSet<&str> = reasons.iter().map(|r| r.name()).collect();
    assert_eq!(names.len(), reasons.len());
    assert!(
        names.contains("no_execution_seconds")
            && names.contains("factor_refused")
            && names.contains("forced_exit_unpriceable")
    );
    let refusals = [
        ScoreRefusal::ProfilesEmpty,
        ScoreRefusal::ChargesPerProfile {
            profiles: 1,
            charges: 0,
        },
        ScoreRefusal::ProfileOutsideExits { variant: 0 },
        ScoreRefusal::TableMismatch { what: "x" },
        ScoreRefusal::RowOutOfTable { row: 0 },
        ScoreRefusal::OutcomeOutOfRange {
            index: 0,
            outcomes: 0,
        },
        ScoreRefusal::DayNotTradable { day: 0 },
        ScoreRefusal::PricedWithoutQty { row: 0 },
        ScoreRefusal::RiskNotPositive { row: 0 },
        ScoreRefusal::EntryValueNotPositive { row: 0 },
        ScoreRefusal::MoneyOverflow { row: 0 },
        ScoreRefusal::CalendarIndexNotIncreasing { day: 0 },
        ScoreRefusal::CalendarKeyDecreasing {
            day: 0,
            period: "week",
        },
        ScoreRefusal::RungsNotIncreasing { ladder: "net_r" },
    ];
    let names: BTreeSet<&str> = refusals.iter().map(ScoreRefusal::name).collect();
    assert_eq!(names.len(), refusals.len());
    for refusal in &refusals {
        assert!(
            refusal
                .to_string()
                .starts_with(&format!("fib score refused ({})", refusal.name()))
        );
        let _: &dyn std::error::Error = refusal;
    }
    let mut gross = Gross;
    let order = Order {
        stock: 0,
        day: 0,
        buy: true,
        qty: 1,
        value_paisa: 1,
        broker_square_off: true,
    };
    assert_eq!(
        (gross.scale(), gross.add(&order), gross.close_day(0)),
        (1, 0, 0)
    );
}

/// The card's bytes move with every figure.
#[test]
fn a_cards_bytes_move_with_its_figures() {
    let entries = [
        Entry {
            slot: 0,
            day: 100,
            second: at(10, 0),
            side: 0,
            outcome: trade(100, at(10, 0), at(10, 5)),
        },
        Entry {
            slot: 1,
            day: 101,
            second: at(10, 0),
            side: 0,
            outcome: trade(-100, at(10, 0), at(10, 5)),
        },
    ];
    let (table, outcomes) = case(Search::Long, &entries);
    let card = score_small(&table, &outcomes);
    let base = bytes(&card);
    let mut changes: Vec<Card> = Vec::new();
    let mut c = card.clone();
    c.months[0].sum_r += 1.0;
    changes.push(c);
    let mut c = card.clone();
    c.months[0].max_r = None;
    changes.push(c);
    let mut c = card.clone();
    c.months[0].win_money += 1;
    changes.push(c);
    let mut c = card.clone();
    c.months[0].per_side[0].trades += 1;
    changes.push(c);
    let mut c = card.clone();
    c.counts.ties += 1;
    changes.push(c);
    let mut c = card.clone();
    c.streaks.losing_r_now = -2.0;
    changes.push(c);
    let mut c = card.clone();
    c.drawdown.max_drawdown_recovery_day = Some(1);
    changes.push(c);
    let mut c = card.clone();
    c.drawdown.underwater_now_days += 1;
    changes.push(c);
    let mut c = card.clone();
    c.periods[3].flat += 1;
    changes.push(c);
    let mut c = card.clone();
    c.profile.size = TradeSize::Large;
    changes.push(c);
    let mut seen = BTreeSet::from([base]);
    for changed in &changes {
        assert!(seen.insert(bytes(changed)), "{changed:?}");
    }
}
