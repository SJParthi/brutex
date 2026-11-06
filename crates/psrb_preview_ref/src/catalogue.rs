//! The 200 golden scenarios: 85 written by hand, one or two planted extremes each, and
//! 115 drawn from the fixed seeds. Every price is synthetic.

use std::collections::BTreeMap;

use crate::generator::{DayPlan, Draw, SEEDS, Scenario, Seconds, StockPlan, calendar, day, market, with_answer, with_probe};
use crate::market::{Bar, Calendar, DayKind, Stock, Window};
use crate::membership::{Answer, Probe};
use crate::time::{Day, Month};

/// The minute index of `hh:mm` (0 = 09:15).
#[must_use]
pub const fn idx(hh: u32, mm: u32) -> u32 {
    (hh * 60 + mm).saturating_sub(9 * 60 + 15)
}

/// The second of the day of `hh:mm:ss`.
#[must_use]
pub const fn sec(hh: u32, mm: u32, ss: u32) -> u32 {
    hh * 3_600 + mm * 60 + ss
}

/// The months `count` before `first`, oldest first.
fn before(first: Month, count: i32) -> Vec<Month> {
    (1..=count).rev().map(|n| first.plus(-n)).collect()
}

/// Flat until `from`, then `level + jump` to the close.
fn gap(level: i64, jump: i64, from: u32) -> DayPlan {
    DayPlan::flat(level, 375).level(from, 375, level + jump)
}

/// Flat until `from`, a jump, then `step` paisa a minute in the jump's direction.
fn gap_drift(level: i64, jump: i64, from: u32, step: i64) -> DayPlan {
    gap_steps(level, jump, from, step, 1)
}

/// Flat until `from`, a jump, then `step` paisa every `every` minutes.
fn gap_steps(level: i64, jump: i64, from: u32, step: i64, every: u32) -> DayPlan {
    let mut plan = DayPlan::flat(level, 375);
    for (n, i) in (from..375).enumerate() {
        let steps = u32::try_from(n).unwrap_or(0) / every.max(1);
        let p = level + jump + step * i64::from(steps);
        plan = plan.minute(i, Bar::flat(p, crate::generator::VOLUME));
    }
    plan
}

/// The last close of a plan.
fn last_close(plan: &DayPlan) -> i64 {
    plan.minutes.values().next_back().map_or(0, |b| b.close)
}

/// One stock: flat warm-up months at `base`, then every full session of the window at the
/// running level, the event days replacing it.
fn stock(name: &str, cal: &Calendar, warm: &[Month], window: Window, base: i64, events: &[(Day, DayPlan)]) -> Stock {
    let events: BTreeMap<Day, &DayPlan> = events.iter().map(|(d, p)| (*d, p)).collect();
    let mut plan = StockPlan::new(name).warmup(cal, warm, base);
    let mut level = base;
    for m in window.months() {
        for d in cal.full_days(m.start(), m.end()) {
            if let Some(p) = events.get(&d) {
                plan = plan.day(d, p, true);
                level = last_close(p);
            } else {
                plan = plan.day(d, &DayPlan::flat(level, 375), true);
            }
        }
    }
    plan.build()
}

/// GDFL does not hold `month`: the month leaves the list and its days hold no second.
fn drop_gdfl_month(stock: &mut Stock, month: Month) {
    stock.gdfl_months.remove(&month);
    for (_, sd) in stock.days.iter_mut().filter(|(d, _)| d.month() == month) {
        sd.seconds.clear();
    }
}

fn window(y: i32, m: u32, months: i32) -> Window {
    let first = Month::of(y, m);
    Window { first, last: first.plus(months - 1) }
}

fn one(name: &str, about: &str, w: Window, cal: Calendar, stocks: Vec<Stock>) -> Scenario {
    Scenario { name: name.to_owned(), seed: 0, about: about.to_owned(), market: market(w, cal, stocks) }
}

/// A one-stock scenario: two flat warm-up months, the window month, events on the days.
fn single(name: &str, about: &str, y: i32, m: u32, base: i64, events: &[(Day, DayPlan)]) -> Scenario {
    let cal = calendar(&[]);
    let w = window(y, m, 1);
    let s = stock("S01", &cal, &before(w.first, 2), w, base, events);
    one(name, about, w, cal, vec![s])
}

const P: i64 = 100_000;
const J: i64 = 1_000;

fn exits() -> Vec<Scenario> {
    let d = day(2018, 9, 4);
    vec![
        single("s001-long-gap-drift-time-exit", "a jump up then a 1-paisa drift: long signals on every timeframe, time exits at 15:07:59", 2018, 9, P, &[(d, gap_drift(P, J, 30, 1))]),
        single("s002-short-gap-drift-time-exit", "the mirror: a jump down then a 1-paisa slide, short signals, time exits", 2018, 9, P, &[(d, gap_drift(P, -J, 30, -1))]),
        single("s003-long-flat-after-gap-stop-at-level", "a jump then a flat tape: the ratcheted stop reaches the price and fills at it (low <= stop)", 2018, 9, P, &[(d, gap(P, J, 30))]),
        single(
            "s004-long-gap-through-stop",
            "after the long entry a minute far below the stop: the stop fills in full at that second's low",
            2018,
            9,
            P,
            &[(d, gap_drift(P, J, 30, 1).level(idx(11, 0), idx(11, 1), P - 5 * J))],
        ),
        single(
            "s005-short-gap-through-stop",
            "after the short entry a spike far above the stop: the stop fills at that second's high",
            2018,
            9,
            P,
            &[(d, gap_drift(P, -J, 30, -1).level(idx(11, 0), idx(11, 1), P + 5 * J))],
        ),
        single(
            "s006-long-flip-close-below-band",
            "Zerodha closes far below the lower band at 11:00 while GDFL stays above the stop: a flip, filled at the next second",
            2018,
            9,
            P,
            &[(d, flip_plan(P, J, idx(11, 0), false))],
        ),
        single(
            "s007-short-flip-close-above-band",
            "the mirror: Zerodha closes far above the upper band, GDFL stays below the stop: a short flip",
            2018,
            9,
            P,
            &[(d, flip_plan(P, -J, idx(11, 0), false))],
        ),
        single(
            "s008-stop-and-flip-in-one-second",
            "the flip's fill second also reaches the stop: one exit at that print, counted as a flip",
            2018,
            9,
            P,
            &[(d, flip_plan(P, J, idx(11, 0), true))],
        ),
        single(
            "s009-stop-and-time-exit-in-one-second",
            "the 15:07:59 second's low reaches the stop: the resting stop is taken (Q3 pin)",
            2018,
            9,
            P,
            &[(d, gap_drift(P, J, 30, 1).second(sec(15, 7, 59), Bar::flat(P, 10)))],
        ),
    ]
}

fn exits_late() -> Vec<Scenario> {
    let d = day(2018, 9, 4);
    vec![
        single(
            "s010-square-off-no-second-in-window",
            "no GDFL second from 15:07:59 to 15:09:59: the broker squares off at 15:10:00's worst print (by_broker)",
            2018,
            9,
            P,
            &[(d, drop_range(gap_drift(P, J, 30, 1), idx(15, 7), idx(15, 10)))],
        ),
        single(
            "s011-flip-filled-at-square-off",
            "a flip decided before 15:08 whose first second is at 15:10:00: the broker's square-off (Q4 pin)",
            2018,
            9,
            P,
            &[(d, drop_range(flip_plan(P, J, idx(15, 0), false), idx(15, 1), idx(15, 10)))],
        ),
        single(
            "s012-halted-afternoon-long-unclosable",
            "no GDFL second and padded minutes from 11:00: the long ends unclosable at its lowest low since entry",
            2018,
            9,
            P,
            &[(d, gap_drift(P, J, 30, 1).pad(idx(11, 0), 375).halt(sec(11, 0, 0)))],
        ),
        single(
            "s013-halted-afternoon-short-unclosable",
            "the mirror: a short halted from 11:00 ends at its highest high since entry",
            2018,
            9,
            P,
            &[(d, gap_drift(P, -J, 30, -1).pad(idx(11, 0), 375).halt(sec(11, 0, 0)))],
        ),
        single(
            "s014-entry-is-the-last-second",
            "the entry fills on the day's last traded second: unclosable at that second's worst print",
            2018,
            9,
            P,
            &[(d, gap(P, J, idx(14, 0)).pad(idx(14, 1), 375).second(sec(14, 1, 30), Bar::new(P + J, P + J + 9, P + J - 9, P + J, 10)).halt(sec(14, 1, 31)))],
        ),
        single(
            "s015-late-fill-over-60-seconds",
            "no GDFL second for two minutes after the signal: the entry fills 121 s late (counted late)",
            2018,
            9,
            P,
            &[(d, gap_drift(P, J, 30, 1).drop_second(sec(9, 46, 1)).drop_second(sec(9, 47, 1)))],
        ),
        single(
            "s016-entry-not-filled",
            "the GDFL tape stops at the signal close: entry not filled, the stock stays flat",
            2018,
            9,
            P,
            &[(d, gap(P, J, 30).halt(sec(9, 46, 0)))],
        ),
        single(
            "s017-no-ratio-minute",
            "no GDFL second before the signal on that day: the stop cannot be mapped (no ratio), not taken",
            2018,
            9,
            P,
            &[(d, drop_range(gap(P, J, 30), 0, 31))],
        ),
        single("s018-zero-quantity-above-5-lakh", "a Rs 6 lakh share: 5,00,000 / price = 0 shares, zero quantity", 2018, 9, 60_000_000, &[(d, gap(60_000_000, 600_000, 30))]),
    ]
}

/// Drops every drawn second of minutes `from..to` (index range).
fn drop_range(mut plan: DayPlan, from: u32, to: u32) -> DayPlan {
    for i in from..to {
        plan = plan.drop_second(sec(9, 15, 1) + 60 * i);
    }
    plan
}

/// A trade that flips at minute `at`: Zerodha closes far across the band there, while the
/// GDFL second inside that minute stays on the trade's side; when `through`, the next
/// second also reaches the stop.
fn flip_plan(level: i64, jump: i64, at: u32, through: bool) -> DayPlan {
    let up = jump > 0;
    let step = if up { 1 } else { -1 };
    let base = gap_drift(level, jump, 30, step);
    let here = base.minutes.get(&at).map_or(level + jump, |b| b.close);
    let crash = if up { level - 3 * jump.abs() } else { level + 3 * jump.abs() };
    let next = if through { crash } else { here };
    let at_second = sec(9, 15, 1) + 60 * at;
    base.minute(at, Bar::flat(crash, crate::generator::VOLUME))
        .second(at_second, Bar::flat(here, 10))
        .minute(at + 1, Bar::flat(next, crate::generator::VOLUME))
}

fn signal_edges() -> Vec<Scenario> {
    let d = day(2018, 9, 4);
    vec![
        single(
            "s019-close-at-2pow32-accepted-above-refused",
            "price edges: a close of 2^32 paisa folds; one of 2^32 + 1 is refused, a hole that taints every reading",
            2018,
            9,
            P,
            &[(d, gap(P, J, 30).minute(idx(10, 0), Bar::flat((1_i64 << 32) + 1, 1_000)).minute(idx(10, 30), Bar::flat(1_i64 << 32, 1_000)))],
        ),
        single("s020-one-rupee-share", "a Rs 1 share with 1-paisa moves: 5,00,000 shares, every charge at its smallest", 2018, 9, 100, &[(d, gap_drift(100, 5, 30, 0))]),
        single(
            "s021-corrupt-candle-hole",
            "a minute with high below low: the candle is refused, a hole, and the jump right after it decides nothing",
            2018,
            9,
            P,
            &[(d, gap(P, J, 30).minute(29, Bar::new(P, P - 10, P + 10, P, 1_000)))],
        ),
        single(
            "s022-missing-minute-taints",
            "a missing minute just before the jump: a hole, then every reading tainted for its own period",
            2018,
            9,
            P,
            &[(d, gap(P, J, 30).drop_minute(28))],
        ),
        single(
            "s023-padded-minutes-are-not-holes",
            "zero-volume padded minutes before the jump: no hole, no taint, the signal stands",
            2018,
            9,
            P,
            &[(d, gap_drift(P, J, 30, 1).pad(20, 30))],
        ),
        single("s024-one-candle-day", "a day holding only its 09:15 minute: one clean 1-minute candle, every other bucket a hole", 2018, 9, P, &[(d, DayPlan::flat(P, 1))]),
        single(
            "s025-signal-on-the-last-entry-minute",
            "the jump in the 15:06 minute: the 1-minute signal at T 15:07:00 enters, then the 15:07:59 order",
            2018,
            9,
            P,
            &[(d, gap(P, J, idx(15, 6)))],
        ),
        single(
            "s026-cut-by-1508",
            "the jump in the 15:07 minute: the 1-minute signal at T 15:08:00 is cut by 15:08",
            2018,
            9,
            P,
            &[(d, gap(P, J, idx(15, 7)))],
        ),
        single(
            "s027-reentry-after-a-stop",
            "a jump, a stop-out on a dip, a second jump later: the stock re-enters with no cap",
            2018,
            9,
            P,
            &[(d, gap(P, J, 30).level(idx(11, 0), idx(12, 0), P + J - 400).level(idx(12, 0), 375, P + 3 * J))],
        ),
        single(
            "s028-zero-volume-seconds-used-as-stored",
            "GDFL seconds with volume 0 (a Rule T break) are used as stored for the entry and the exits",
            2018,
            9,
            P,
            &[(d, gap_drift(P, J, 30, 1).second(sec(9, 46, 1), Bar::new(P + J, P + J + 7, P + J, P + J, 0)))],
        ),
        Scenario {
            name: "s029-split-day-ratio-5".to_owned(),
            seed: 0,
            about: "Zerodha adjusted, GDFL raw at 5 times: the stop maps through the ratio 5 and the quantity sizes from the raw fill"
                .to_owned(),
            market: {
                let cal = calendar(&[]);
                let w = window(2018, 9, 1);
                let mut s = stock("S01", &cal, &before(w.first, 2), w, P, &[]);
                s.days.insert(d, gap_drift(P, J, 30, 1).factor(5).build());
                market(w, cal, vec![s])
            },
        },
    ]
}

fn fallbacks() -> Vec<Scenario> {
    let d = day(2018, 9, 4);
    let cal = calendar(&[]);
    let w = window(2018, 9, 1);
    let warm = before(w.first, 2);
    let base = |events: &[(Day, DayPlan)]| stock("S01", &cal, &warm, w, P, events);
    let w2 = window(2018, 9, 2);
    let mut missing = stock("S01", &cal, &warm, w2, P, &[(day(2018, 10, 3), gap_drift(P, J, 30, 1))]);
    drop_gdfl_month(&mut missing, Month::of(2018, 10));
    let mut refused = base(&[(d, gap_drift(P, J, 30, 1))]);
    refused.gdfl_refused.insert(w.first);
    let no_seconds = base(&[(d, gap_drift(P, J, 30, 1).seconds_rule(Seconds::None))]);
    let halt = base(&[(d, gap_drift(P, J, 30, 1).pad(idx(11, 0), 375).seconds_rule(Seconds::None))]);
    let late = base(&[(d, gap(P, J, idx(15, 6)).seconds_rule(Seconds::None))]);
    let padded_entry = base(&[(d, gap_drift(P, J, 30, 1).pad(31, 33).seconds_rule(Seconds::None))]);
    let vendor = {
        let w2 = window(2019, 5, 1);
        let s = stock("S01", &cal, &before(w2.first, 2), w2, P, &[(day(2019, 5, 30), gap_drift(P, J, 30, 1))]);
        one("s035-vendor-late-day-2019-05-30", "2019-05-30: every stock fills on minutes though GDFL seconds exist", w2, calendar(&[]), vec![s])
    };
    vec![
        one("s030-gdfl-month-missing-fallback", "GDFL holds September but not October: every October fill on Zerodha minutes (high/low, slot second = start + 59)", w2, cal.clone(), vec![missing]),
        one("s031-gdfl-month-refused-fallback", "the GDFL month refuses to open: a fallback day by reason", w, cal.clone(), vec![refused]),
        one("s032-day-without-seconds-fallback", "Zerodha minutes but no GDFL second that day: a fallback day", w, cal.clone(), vec![no_seconds]),
        one("s033-fallback-unclosable", "a fallback day whose minutes are padded from 11:00: unclosable at the worst minute since entry", w, cal.clone(), vec![halt]),
        one("s034-fallback-entry-on-the-1507-minute", "fallback entry at T 15:07:00 on the 15:07 minute (slot 15:07:59), then the time rule on 15:08", w, cal.clone(), vec![late]),
        vendor,
        one("s036-fallback-padded-entry-minute", "fallback: the minutes after the signal are padded, the entry takes the next traded minute", w, cal, vec![padded_entry]),
    ]
}

fn calendar_cases() -> Vec<Scenario> {
    let mut out = Vec::new();
    {
        let cal = calendar(&[]);
        let w = window(2021, 2, 1);
        let s = stock("S01", &cal, &before(w.first, 2), w, P, &[(day(2021, 2, 24), gap_drift(P, J, 30, 1))]);
        out.push(one("s037-outage-day-2021-02-24", "2021-02-24 never trades; its candles still fold (the jump decides nothing)", w, cal, vec![s]));
    }
    {
        let cal = calendar(&[(day(2018, 9, 13), DayKind::Closed), (day(2018, 9, 14), DayKind::Muhurat), (day(2018, 9, 17), DayKind::Short)]);
        let w = window(2018, 9, 1);
        let mut s = stock("S01", &cal, &before(w.first, 2), w, P, &[(day(2018, 9, 18), gap_drift(P, J, 30, 1))]);
        for d in [day(2018, 9, 13), day(2018, 9, 14), day(2018, 9, 17)] {
            s.days.insert(d, gap_drift(P, -J, 30, -1).build());
        }
        out.push(one("s038-holiday-muhurat-short-session", "a closed weekday, a Muhurat day and a short session hold minutes: none folds or trades; the day after reads the last full session", w, cal, vec![s]));
    }
    let d = day(2018, 9, 4);
    let cal = calendar(&[]);
    let w = window(2018, 9, 1);
    let warm = before(w.first, 2);
    let s = stock("S01", &cal, &warm, w, P, &[(d, gap_drift(P, J, 30, 1))]);
    let m = market(w, cal.clone(), vec![s.clone()]);
    out.push(Scenario { name: "s039-membership-unknown-day".to_owned(), seed: 0, about: "the folder answers unknown on the signal day: not traded, counted per stock".to_owned(), market: with_answer(m.clone(), "S01", d, Answer::Unknown) });
    out.push(Scenario { name: "s040-membership-no-day".to_owned(), seed: 0, about: "not a member on the signal day: not traded".to_owned(), market: with_answer(m.clone(), "S01", d, Answer::No) });
    let failing = Probe { opened: true, reliance_first: Answer::Yes, reliance_last: Answer::Yes, syrma_last: Answer::Yes };
    out.push(Scenario {
        name: "s041-probe-fails-todays-list".to_owned(),
        seed: 0,
        about: "the probe fails (SYRMA answers Ok(true)): today's list on every day, the dated 'no' is ignored".to_owned(),
        market: with_probe(with_answer(m.clone(), "S01", d, Answer::No), failing),
    });
    let mut carry = with_answer(m, "S01", day(2018, 9, 3), Answer::No);
    carry.membership.carry_from = Some(day(2018, 9, 3));
    out.push(Scenario { name: "s042-membership-carry-from".to_owned(), seed: 0, about: "days after the folder's last day ask that day: its 'no' blocks the signal day".to_owned(), market: carry });
    let mut withheld = s.clone();
    withheld.withheld.insert(d);
    out.push(one("s043-withheld-day", "the fold withholds the signal day: every scheduled bucket a hole, no bar folded", w, cal.clone(), vec![withheld]));
    let mut off_grid = s.clone();
    let mut bad = DayPlan::flat(P + J + 344, 375).build();
    for m0 in bad.minutes.iter_mut().skip(10).take(1) {
        m0.start += 30;
    }
    off_grid.days.insert(day(2018, 9, 5), bad);
    out.push(one("s044-fold-error-month", "one minute stamped 09:25:30: the whole month's fold refuses, every bucket a hole", w, cal.clone(), vec![off_grid]));
    {
        let w2 = window(2026, 8, 1);
        let cal2 = calendar(&[]);
        let mut st = stock("S01", &cal2, &before(w2.first, 2), w2, P, &[]);
        for dd in cal2.full_days(w2.first.start(), w2.first.end()) {
            let plan = if dd == day(2026, 8, 4) { gap_drift(P, J, 30, 1) } else { DayPlan::flat(P, 375) };
            st.days.insert(dd, plan.truncate(360).build());
        }
        out.push(one("s045-auction-eligible-days-2026-08", "minutes end at 15:14 from 2026-08-03: 15:15 continuous close, 360-minute schedule", w2, cal2, vec![st]));
    }
    {
        let w3 = window(2026, 7, 2);
        let cal3 = calendar(&[]);
        let mut st = stock("S01", &cal3, &before(w3.first, 2), w3, P, &[(day(2026, 7, 31), gap_drift(P, J, 30, 1))]);
        st.days.insert(day(2026, 8, 3), gap_drift(P + J + 344, J, 30, 1).truncate(360).build());
        out.push(one("s046-auction-switch-eve", "2026-07-31 a full session, 2026-08-03 auction-eligible: both session shapes in one run", w3, cal3, vec![st]));
    }
    {
        let mut st = stock("S01", &cal, &[], w, P, &[(d, gap_drift(P, J, 30, 1))]);
        st.zerodha_months.clear();
        st.zerodha_months.insert(Month::of(2018, 7));
        st.zerodha_months.insert(Month::of(2018, 10));
        out.push(one("s047-trading-start-month-absent", "the trading start month has no Zerodha month: refused for the stock (Q1 pin)", w, cal.clone(), vec![st]));
    }
    {
        let w4 = window(2018, 9, 2);
        let mut st = stock("S01", &cal, &before(w4.first, 2), w4, P, &[(day(2018, 10, 3), gap_drift(P, J, 30, 1))]);
        st.gdfl_months.remove(&Month::of(2018, 9));
        out.push(one("s048-late-gdfl-start", "GDFL starts a month after Zerodha: the trading span starts at October's first session", w4, cal.clone(), vec![st]));
    }
    {
        let mut st = stock("S01", &cal, &before(w.first, 2), w, P, &[(d, gap_drift(P, J, 30, 1))]);
        st.zerodha_months.insert(Month::of(2018, 3));
        st.days.insert(day(2018, 3, 5), DayPlan::flat(99_500, 375).seconds_rule(Seconds::None).build());
        out.push(one("s049-zerodha-gap-warmup-from-contiguous-run", "a foreign Zerodha month before a gap is never folded: warm-up from the contiguous run", w, cal.clone(), vec![st]));
    }
    {
        let st = stock("S01", &cal, &[], w, P, &[(day(2018, 9, 12), gap_drift(P, J, 30, 1))]);
        out.push(one("s050-no-warmup-cold-ema", "no warm-up month: VIDEO-ORIGINAL trades once its three readings warm; AUTHOR-V2 and VIDEO-SCREEN wait for EMA 200", w, cal, vec![st]));
    }
    out
}

fn books() -> Vec<Scenario> {
    let d = day(2018, 9, 4);
    let cal = calendar(&[]);
    let w = window(2018, 9, 1);
    let warm = before(w.first, 2);
    let mut out = Vec::new();
    let names: Vec<String> = (1..=22).map(|i| format!("S{i:02}")).collect();
    let same: Vec<Stock> = names.iter().map(|n| stock(n, &cal, &warm, w, P, &[(d, gap_steps(P, J, 30, 1, 25))])).collect();
    out.push(one("s051-twenty-slots-full-22-ties", "22 stocks jump at one instant: 20 admitted in symbol order, 2 skipped 'slots full'", w, cal.clone(), same));
    let mut mixed: Vec<Stock> = names.iter().take(20).map(|n| stock(n, &cal, &warm, w, P, &[(d, gap_steps(P, J, 30, 1, 25))])).collect();
    mixed.push(stock("S21", &cal, &warm, w, P, &[(d, gap_steps(P, J, 31, 1, 25))]));
    mixed.push(stock("S22", &cal, &warm, w, P, &[(d, gap_steps(P, J, 31, 1, 25))]));
    out.push(one("s052-slots-full-then-while-open", "20 open, then later signals: their own stocks while open, two newcomers slots full", w, cal.clone(), mixed));
    let both = stock("S01", &cal, &warm, w, P, &[(d, gap_drift(P, J, 30, 1).level(idx(10, 0), 375, P - 2 * J))]);
    out.push(one("s053-both-view-long-then-short", "a long open, then a short signal on the same stock: BOTH counts it while open, SHORT takes it", w, cal.clone(), vec![both]));
    let byte_names = ["AB-C", "AB&C", "AB&CD", "ABC"];
    let ordered: Vec<Stock> = byte_names.iter().map(|n| stock(n, &cal, &warm, w, P, &[(d, gap_steps(P, J, 30, 1, 25))])).collect();
    out.push(one("s054-symbol-byte-order-ties", "ties at one instant ordered by bytes: AB&C < AB&CD < AB-C < ABC", w, cal.clone(), ordered));
    out.push(exact_month("s055-month-exactly-2-5000-percent-passes", 5_000_000));
    out.push(exact_month("s056-month-2-4999-percent-fails", 4_999_800));
    {
        let w3 = window(2018, 9, 3);
        let st = stock("S01", &cal, &warm, w3, P, &[(day(2018, 10, 3), gap_drift(P, J, 30, 1))]);
        out.push(one("s057-no-trade-months", "three judged months, trades in October only: September and November are 0 bp and fail", w3, cal.clone(), vec![st]));
    }
    {
        let w2 = window(2018, 9, 2);
        let st = stock(
            "S01",
            &cal,
            &warm,
            w2,
            P,
            &[(day(2018, 9, 4), gap(P, J, 30).level(idx(10, 0), 375, P - 20 * J)), (day(2018, 10, 3), gap_drift(P - 20 * J, J, 30, 3))],
        );
        out.push(one("s058-loss-month-then-recovery", "a gap-through loss in September and a gain in October: a drawdown with its peak and low days", w2, cal.clone(), vec![st]));
    }
    {
        let w2 = window(2018, 9, 2);
        let crash: Vec<Stock> =
            names.iter().map(|n| stock(n, &cal, &warm, w2, P, &[(d, gap(P, J, 30).level(idx(10, 0), 375, 1))])).collect();
        out.push(one("s059-account-at-or-below-zero", "20 longs crash to one paisa: September loses more than the account, October starts at or below zero", w2, cal.clone(), crash));
    }
    {
        let st = stock(
            "S01",
            &cal,
            &warm,
            w,
            P,
            &[(day(2018, 9, 4), gap(P, J, 30).level(idx(10, 0), 375, P)), (day(2018, 9, 6), gap(P, J, 30).level(idx(10, 0), 375, P))],
        );
        out.push(one("s060-equal-drawdowns-first-dates-kept", "two equal falls from one peak: the first one's dates are kept (Q9 pin)", w, cal.clone(), vec![st]));
    }
    {
        let w0 = window(2018, 8, 1);
        let st = stock("S01", &cal, &before(w0.first, 2), w0, P, &[(day(2018, 8, 7), gap_drift(P, J, 30, 1))]);
        out.push(one("s061-charge-refusal-outside-cost-window", "a trade on 2018-08-07, before the cost table's 2018-09-03: the run stops by name", w0, cal.clone(), vec![st]));
    }
    {
        let a = stock("S01", &cal, &warm, w, P, &[(d, gap_drift(P, J, 30, 1))]);
        out.push(one("s062-two-stocks-one-spelling", "two stocks spelled S01 with signals at one instant: the merge refuses the duplicate key by name", w, cal, vec![a.clone(), a]));
    }
    out
}

/// One long trade whose month net is exactly `target` paisa (5,000,000 is 2.5000% of the
/// account; 4,999,800 is 2.4999%): the entry and the 15:07:59 exit prices are searched.
fn exact_month(name: &str, target: i128) -> Scenario {
    let d = day(2018, 9, 4);
    let cal = calendar(&[]);
    let w = window(2018, 9, 1);
    let (entry, exit) = EXACT.iter().find(|(t, _)| *t == target).map_or((0, 0), |(_, pair)| *pair);
    let mut plan = gap_drift(P, entry - P, 30, 1);
    for i in 30..idx(15, 8) {
        plan = plan.second(sec(9, 15, 1) + 60 * i, Bar::flat(entry, 10));
    }
    plan = plan.second(sec(15, 7, 59), Bar::flat(exit, 10));
    let st = stock("S01", &cal, &before(w.first, 2), w, P, &[(d, plan)]);
    Scenario {
        name: name.to_owned(),
        seed: 0,
        about: format!(
            "one long trade per long row, every timeframe: GDFL flat at {entry} from the jump, the 15:07:59 second at {exit}: a net of exactly {target} paisa on the Rs 20,00,000 start"
        ),
        market: market(w, cal, vec![st]),
    }
}

/// The pairs [`solve_exact`] finds on 2018-09-04, pinned so the catalogue builds fast; a
/// test re-derives them.
pub const EXACT: [(i128, (i64, i64)); 2] = [(5_000_000, (112_360, 123_698)), (4_999_800, (121_906, 134_184))];

/// The first (entry, exit) pair from Rs 1,010 up whose long net is `target`, searching
/// entries to Rs 1,250 and, for each, the exits around the target's gross.
#[must_use]
pub fn solve_exact(d: Day, target: i128) -> Option<(i64, i64)> {
    (101_000..125_000).find_map(|entry| {
        let q = crate::sizing::qty(entry);
        let lo = entry + i64::try_from(target / i128::from(q)).unwrap_or(0);
        (lo..lo + 300).find(|exit| {
            let orders = crate::costs::trade_orders(true, q, entry, *exit, false);
            let net = crate::costs::price_trade(&orders, d)
                .map(|p| crate::costs::gross_and_net(true, q, entry, *exit, p.headline).1);
            net == Ok(target)
        })
        .map(|exit| (entry, exit))
    })
}

fn eras() -> Vec<Scenario> {
    let days = [
        (2018, 9, 3, "window start: SEBI Rs 15/cr, NSE Rs 325/cr, the worst state's stamp"),
        (2019, 3, 29, "the last day of SEBI Rs 15/cr"),
        (2019, 4, 1, "the first day of SEBI Rs 10/cr"),
        (2020, 5, 29, "the last day before the SEBI 50% relief"),
        (2020, 6, 1, "the first day of SEBI Rs 5/cr"),
        (2020, 6, 30, "the last day of the state stamp on both sides"),
        (2020, 7, 1, "the first day of 0.003% stamp on the buy"),
        (2020, 12, 31, "the last day of NSE Rs 325/cr"),
        (2021, 1, 1, "the first day of NSE Rs 345/cr"),
        (2021, 4, 1, "the first day of SEBI back at Rs 10/cr"),
        (2022, 7, 15, "the last trading day with no GST on the SEBI fee"),
        (2022, 7, 18, "the first day of GST on the SEBI fee"),
        (2023, 3, 31, "the last day of NSE Rs 345 and IPFT Rs 0.01"),
        (2023, 4, 3, "the first trading day of NSE Rs 325 and IPFT Rs 10"),
        (2024, 3, 28, "the last trading day of NSE Rs 325 + 10"),
        (2024, 4, 1, "the first day of NSE Rs 322 + 10"),
        (2024, 9, 30, "the last day of the slab ladders"),
        (2024, 10, 1, "the first day of NSE flat Rs 297 + 10"),
        (2026, 2, 27, "the last day of NSE Rs 297 + 10"),
        (2026, 3, 2, "the first trading day of NSE Rs 306.99 + 0.01"),
        (2026, 10, 30, "the cost table's last priced day"),
    ];
    days.iter()
        .enumerate()
        .map(|(i, (y, m, dd, why))| {
            let d = day(*y, *m, *dd);
            let up = i % 2 == 0;
            let jump = if up { J } else { -J };
            let step = if up { 2 } else { -2 };
            let base = 50_000 + 12_345 * i64::try_from(i).unwrap_or(0);
            let mut s = single(&format!("s{:03}-era-{y:04}-{m:02}-{dd:02}", 63 + i), &format!("{why}: a {} trade priced on both clocks", if up { "long" } else { "short" }), *y, *m, base, &[(d, gap_drift(base, jump, 30, step))]);
            if *y == 2026 && *m == 10 {
                s.about.push_str(" (the window's last month)");
            }
            s
        })
        .collect()
}

/// The hand scenarios, in name order.
#[must_use]
pub fn hand() -> Vec<Scenario> {
    let mut out = exits();
    out.extend(exits_late());
    out.extend(signal_edges());
    out.extend(fallbacks());
    out.extend(calendar_cases());
    out.extend(books());
    out.extend(eras());
    out
}

/// One seeded random scenario: 1..=3 stocks (sometimes 22), 1..=2 judged months over the
/// cost eras, flat warm-up, and per stock a few event days mixing jumps, drifts, dips,
/// spikes, halts, padded and missing minutes, dropped and zero-volume seconds, and
/// fallback days.
#[must_use]
pub fn random(story: usize, number: usize) -> Scenario {
    let seed = SEEDS.get(story % 4).copied().unwrap_or(1) ^ (u64::try_from(story).unwrap_or(0) << 20);
    let mut draw = Draw::new(seed);
    let months = [
        (2018, 9),
        (2019, 2),
        (2019, 11),
        (2020, 3),
        (2020, 8),
        (2021, 6),
        (2022, 1),
        (2022, 9),
        (2023, 5),
        (2023, 11),
        (2024, 6),
        (2025, 1),
        (2025, 9),
        (2026, 4),
        (2026, 9),
    ];
    let pick = usize::try_from(draw.int(0, 14)).unwrap_or(0);
    let (year, month) = months.get(pick).copied().unwrap_or((2018, 9));
    let judged = window(year, month, if draw.chance(1, 4) { 2 } else { 1 });
    let cal = calendar(&[]);
    let crowd = draw.chance(1, 25);
    let count = if crowd { 22 } else { draw.index(1, 3) };
    let warm = before(judged.first, i32::try_from(draw.index(0, 2)).unwrap_or(1));
    let full: Vec<Day> = judged.months().iter().flat_map(|mm| cal.full_days(mm.start(), mm.end())).collect();
    let crowd_plan = event_plan(&mut draw, 100_000);
    let crowd_at = usize::try_from(draw.int(1, 4)).unwrap_or(1);
    let mut stocks = Vec::new();
    for ordinal in 0..count {
        let name = format!("R{:02}", ordinal + 1);
        let base = if crowd { 100_000 } else { draw.int(5_000, 500_000) };
        let mut events = Vec::new();
        for (at, dd) in full.iter().copied().enumerate().skip(1) {
            let planted = if crowd { at == crowd_at } else { events.len() < 3 && draw.chance(1, 7) };
            if planted {
                let plan = if crowd { crowd_plan.clone() } else { event_plan(&mut draw, base) };
                events.push((dd, plan));
            }
        }
        let mut st = stock(&name, &cal, &warm, judged, base, &events);
        if draw.chance(1, 10) {
            drop_gdfl_month(&mut st, judged.first);
        }
        if draw.chance(1, 15) {
            st.gdfl_refused.insert(judged.last);
        }
        stocks.push(st);
    }
    let mut mk = market(judged, cal, stocks);
    if draw.chance(1, 8) {
        let answer = if draw.chance(1, 2) { Answer::Unknown } else { Answer::No };
        for dd in full.iter().copied().skip(2).take(1) {
            mk = with_answer(mk, "R01", dd, answer);
        }
    }
    Scenario {
        name: format!("s{number:03}-random-{}", judged.first.label()),
        seed,
        about: format!(
            "seeded random story {story}: {count} stock(s), {} judged month(s) from {}",
            judged.months().len(),
            judged.first.label()
        ),
        market: mk,
    }
}

/// A random event day around `base`.
fn event_plan(r: &mut Draw, base: i64) -> DayPlan {
    let up = r.chance(1, 2);
    let size = (base * r.int(3, 30)).div_euclid(1_000).max(1);
    let jump = if up { size } else { -size };
    let at = r.index(20, 340);
    let step = if r.chance(1, 2) { 0 } else { (base.div_euclid(20_000)).max(1) * if up { 1 } else { -1 } };
    let mut plan = gap_steps(base, jump, at, step, r.index(1, 15));
    if r.chance(1, 3) {
        let dip = r.index(at + 2, 370);
        let depth = size * r.int(1, 6);
        let to = if up { base + jump - depth } else { base + jump + depth };
        plan = plan.level(dip, dip + r.index(1, 30), to.max(1));
    }
    if r.chance(1, 5) {
        let from = r.index(at + 1, 370);
        plan = plan.pad(from, (from + r.index(1, 60)).min(375));
    }
    if r.chance(1, 6) {
        plan = plan.drop_minute(r.index(0, 374));
    }
    if r.chance(1, 6) {
        let h = r.index(at + 1, 374);
        plan = plan.pad(h, 375).halt(sec(9, 15, 0) + 60 * h);
    }
    if r.chance(1, 5) {
        let s0 = sec(9, 15, 1) + 60 * r.index(at, 374);
        plan = plan.drop_second(s0).drop_second(s0 + 60);
    }
    if r.chance(1, 8) {
        plan = plan.seconds_rule(Seconds::None);
    }
    if r.chance(1, 6) {
        let i = r.index(at, 374);
        let p = plan.minutes.get(&i).map_or(base, |b| b.close);
        plan = plan.minute(i, Bar::new(p, p + size.div_euclid(3).max(1), (p - size.div_euclid(3)).max(1), p, 2_000));
    }
    plan
}

/// All 200 scenarios, in name order.
#[must_use]
pub fn all() -> Vec<Scenario> {
    let mut out = hand();
    let first = out.len() + 1;
    let n = 200usize.saturating_sub(out.len());
    out.extend((0..n).map(|i| random(i, first + i)));
    out
}

#[cfg(test)]
#[path = "catalogue_tests.rs"]
mod tests;
