#![cfg(test)]
//! The stall list's bound, its idempotence and its exit (D-0949).
//!
//! Split from `autopilot.rs`'s own test module so these can be read as one
//! argument: a stalled month is ONE entry however often it fails, its retry
//! allowance is spent once and never renewed, and it leaves the list when the
//! month completes.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

fn month(y: u16, m: u8) -> YearMonth {
    YearMonth::new(y, m).unwrap()
}

fn groww(at: YearMonth) -> FeedState {
    FeedState::new(
        pull::vendor::Feed::Groww,
        brutex_core::vendor::Vendor::Groww,
        at,
    )
}

fn failed() -> TickOutcome {
    TickOutcome {
        attempted: 773,
        reached: 773,
        stored: 0,
        reason: Some(String::from("operation timed out")),
        complete: false,
        stopped: false,
        journal_error: None,
        credential: None,
    }
}

fn completed() -> TickOutcome {
    TickOutcome {
        attempted: 773,
        reached: 773,
        stored: 10,
        reason: None,
        complete: true,
        stopped: false,
        journal_error: None,
        credential: None,
    }
}

/// Drive one month to its stall, exactly as `settle` does: observe until
/// `Next::Stall`, then step the frontier past it. Returns how many failed
/// ticks it took.
fn stall_out(state: &mut FeedState) -> u32 {
    let mut ticks = 0u32;
    loop {
        ticks += 1;
        assert!(ticks <= 64, "a month must stall within its bound");
        if let Next::Stall { .. } = state.observe(&failed()) {
            if let Some(next) = month_after(state.frontier) {
                state.frontier = next;
            }
            return ticks;
        }
    }
}

/// **W1-api1-8. A reconsidered month that fails again stays ONE stall, and
/// its allowance is not renewed.**
///
/// Before D-0949 `observe` pushed a fresh `Stall { retried: 0, .. }` every
/// time the month burned its attempts, so each reconsideration minted a new
/// entry with a full allowance: the list grew by one per failure for the life
/// of the process and "9 attempts per stalled month" was not enforced. This
/// counts every vendor-bound attempt the month is given, across as many
/// recheck intervals as anybody could wait, and requires the documented
/// arithmetic exactly.
#[test]
fn a_reconsidered_month_that_fails_again_is_one_stall_and_its_allowance_is_not_renewed() {
    let target = month(2021, 3);
    let mut feeds = vec![groww(target)];
    let mut ticks = stall_out(&mut feeds[0]);
    assert_eq!(feeds[0].stalls.len(), 1);

    let mut clock: i64 = 1_000_000;
    // First sighting stamps.
    assert!(reconsider(&mut feeds, clock).is_none());
    let mut reconsidered = 0u32;
    for _ in 0..40 {
        clock = clock.saturating_add(STALL_RECHECK_SECS);
        if reconsider(&mut feeds, clock).is_some() {
            reconsidered += 1;
            assert_eq!(feeds[0].frontier, target);
            ticks += stall_out(&mut feeds[0]);
        }
        assert_eq!(
            feeds[0].stalls.len(),
            1,
            "one month, one entry, however often it fails: {:?}",
            feeds[0].stalls
        );
    }
    assert_eq!(reconsidered, u32::from(STALL_RETRIES));
    assert_eq!(
        ticks,
        u32::from(MAX_MONTH_ATTEMPTS) * (1 + u32::from(STALL_RETRIES)),
        "the 9-attempt bound the doc comments state, measured"
    );
    let stall = &feeds[0].stalls[0];
    assert_eq!(stall.month, target);
    assert_eq!(stall.retried, STALL_RETRIES, "spent, and still saying so");
    assert_eq!(
        u32::from(stall.attempts),
        ticks,
        "the entry counts every attempt the month was given"
    );
    let note = stall_note(&feeds);
    assert!(note.contains("1 stalled month(s)"), "{note}");
    assert!(note.contains("0 still to be reconsidered"), "{note}");
    assert!(note.contains("1 whose allowance is SPENT"), "{note}");
}

/// **A stalled month leaves the list when it completes, and only that one.**
///
/// The comment in the reconsideration test said the stall "leaves only when
/// the month actually completes"; until D-0949 nothing removed it, so a month
/// that succeeded on its second try was still reported as stalled and still
/// counted towards `stall_note` for the life of the process.
#[test]
fn a_stalled_month_that_completes_leaves_the_list_and_no_other_does() {
    let mut feeds = vec![groww(month(2021, 3))];
    stall_out(&mut feeds[0]); // 2021-03
    stall_out(&mut feeds[0]); // 2021-04
    assert_eq!(feeds[0].stalls.len(), 2);

    let mut clock: i64 = 5_000;
    assert!(reconsider(&mut feeds, clock).is_none());
    clock = clock.saturating_add(STALL_RECHECK_SECS);
    assert!(reconsider(&mut feeds, clock).is_some());
    assert_eq!(feeds[0].frontier, month(2021, 3), "the oldest goes first");
    assert_eq!(feeds[0].observe(&completed()), Next::Advance);
    assert_eq!(feeds[0].stalls.len(), 1, "{:?}", feeds[0].stalls);
    assert_eq!(feeds[0].stalls[0].month, month(2021, 4), "the other stays");

    // A COMPLETION WITH NO STALL FOR ITS MONTH CHANGES NOTHING, and doing it
    // again is the same answer (CLAUDE.md §3 rule 5).
    feeds[0].frontier = month(2022, 1);
    for _ in 0..3 {
        assert_eq!(feeds[0].observe(&completed()), Next::Advance);
        assert_eq!(feeds[0].stalls.len(), 1);
    }
    assert!(stall_note(&feeds).contains("1 stalled month(s)"));
}

/// **The list is bounded by the months a feed can owe, not by the failures.**
///
/// Every month from 2020-01 to 2026-08 stalls, three full passes over the
/// calendar. The list length is the cost `reconsider`, `stall_note` and
/// `Status::json` pay, so it is the operation count this measures: one entry
/// per month after the first pass, and the same after the third.
#[test]
fn the_stall_list_is_one_entry_per_month_however_many_passes_fail() {
    let first = month(2020, 1);
    let last = month(2026, 8);
    let months = ordinal(last) - ordinal(first) + 1;
    let mut state = groww(first);
    for pass in 0..3 {
        state.frontier = first;
        while ordinal(state.frontier) <= ordinal(last) {
            stall_out(&mut state);
        }
        assert_eq!(
            state.stalls.len(),
            usize::try_from(months).unwrap(),
            "pass {pass}: one entry per month"
        );
    }
    // Ordered by first stall, and no month twice.
    let mut seen: Vec<u32> = state.stalls.iter().map(|s| ordinal(s.month)).collect();
    let before = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), before);
    // The attempt count saturates rather than wrapping at u8.
    assert!(state.stalls.iter().all(|s| s.attempts == 9));
}

/// **The attempt count saturates at `u8::MAX`, and the entry stays single.**
#[test]
fn a_stall_attempt_count_saturates_rather_than_wrapping() {
    let mut state = groww(month(2020, 1));
    for _ in 0..200 {
        state.frontier = month(2020, 1);
        stall_out(&mut state);
    }
    assert_eq!(state.stalls.len(), 1);
    assert_eq!(state.stalls[0].attempts, u8::MAX);
}

// ------------------------------------------------------------ W1-api1-7

const DAY: pull::vendor::Granularity = pull::vendor::Granularity::Day1;
const MINUTE: pull::vendor::Granularity = pull::vendor::Granularity::Minute1;

/// **W1-api1-7. A day pass that advances the frontier skips no minute
/// month.**
///
/// One frontier used to serve both rungs: six day months completing moved it
/// to 2020-07, and the next minute pass scanned upward from there, so minute
/// 2020-01 to 2020-06 were never examined for the life of the process. Each
/// rung now keeps its own frontier and its own per-month counters.
#[test]
fn each_rung_keeps_its_own_frontier_and_counters() {
    let mut s = groww(month(2020, 1));
    s.enter(DAY);
    for _ in 0..6 {
        assert_eq!(s.observe(&completed()), Next::Advance);
        s.frontier = month_after(s.frontier).unwrap();
    }
    assert_eq!(s.frontier, month(2020, 7));

    s.enter(MINUTE);
    assert_eq!(
        s.frontier,
        month(2020, 1),
        "the minute rung starts where IT stood, not where the day rung got to"
    );
    assert_eq!(s.attempts, 0);
    // A MINUTE FAILURE IS NOT A DAY FAILURE.
    assert!(matches!(s.observe(&failed()), Next::Wait { .. }));
    assert_eq!((s.attempts, s.backoff), (1, 1));

    s.enter(DAY);
    assert_eq!(s.frontier, month(2020, 7));
    assert_eq!((s.attempts, s.dry, s.backoff), (0, 0, 0));
    // RE-ENTERING THE LIVE RUNG CHANGES NOTHING (CLAUDE.md §3 rule 5).
    let before = (s.frontier, s.attempts, s.parked);
    for _ in 0..3 {
        s.enter(DAY);
    }
    assert_eq!((s.frontier, s.attempts, s.parked), before);

    s.enter(MINUTE);
    assert_eq!(s.frontier, month(2020, 1));
    assert_eq!(
        (s.attempts, s.backoff),
        (1, 1),
        "the minute counters survived"
    );
}

/// **The frontier scan itself, over a store whose day rung is held and whose
/// minute rung is empty.** This is the `survey` call shape: each rung's scan
/// starts from that rung's own hint.
#[test]
fn a_held_day_rung_does_not_hide_an_empty_minute_rung_from_the_scan() {
    let floor = Day::new(2020, 1, 1).unwrap();
    let yesterday = Day::new(2020, 12, 31).unwrap();
    let axis = |timeframe| {
        vec![Series {
            contract: None,
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Index,
            symbol: brutex_core::symbol::Symbol::new("NIFTY").unwrap(),
            timeframe,
        }]
    };
    let day_axis = axis(Timeframe::DAY_1);
    let minute_axis = axis(Timeframe::MINUTE_1);
    // The day rung is held through yesterday; the minute rung holds nothing.
    let end: i64 = 1_700_000_000_000_000; // 2023-11, after every month asked
    let held = |key: &EntryKey| (key.timeframe == Timeframe::DAY_1).then_some(end);

    let mut s = groww(month(2020, 1));
    s.enter(DAY);
    let (at, unit) = frontier(held, &day_axis, s.frontier, floor, yesterday);
    assert!(unit.is_none(), "the day rung owes nothing");
    s.frontier = at;
    assert!(ordinal(s.frontier) >= ordinal(month(2020, 12)));

    s.enter(MINUTE);
    let (at, unit) = frontier(held, &minute_axis, s.frontier, floor, yesterday);
    assert_eq!(at, month(2020, 1), "minute 2020-01 is owed and is found");
    assert!(unit.is_some());
}

/// **A stall belongs to its rung.** A minute stall is reconsidered on the
/// minute frontier even while the day rung is live, and a day completion of
/// the same month does not clear it.
#[test]
fn a_stall_is_reconsidered_and_cleared_on_its_own_rung_only() {
    let mut feeds = vec![groww(month(2021, 1))];
    feeds[0].enter(MINUTE);
    stall_out(&mut feeds[0]);
    assert_eq!(feeds[0].stalls[0].rung, MINUTE);
    feeds[0].enter(DAY);
    let day_frontier = feeds[0].frontier;

    let mut clock = 10_000;
    assert!(reconsider(&mut feeds, clock).is_none());
    clock += STALL_RECHECK_SECS;
    let said = reconsider(&mut feeds, clock).expect("due");
    assert!(said.contains("stalled 1min month 2021-01"), "{said}");
    assert_eq!(
        feeds[0].frontier, day_frontier,
        "the day frontier did not move"
    );
    assert_eq!(feeds[0].parked, Place::at(month(2021, 1)));

    // DAY 2021-01 COMPLETING DOES NOT CLEAR THE MINUTE STALL.
    feeds[0].frontier = month(2021, 1);
    assert_eq!(feeds[0].observe(&completed()), Next::Advance);
    assert_eq!(feeds[0].stalls.len(), 1);

    // MINUTE 2021-01 COMPLETING DOES.
    feeds[0].enter(MINUTE);
    assert_eq!(feeds[0].frontier, month(2021, 1));
    assert_eq!(feeds[0].observe(&completed()), Next::Advance);
    assert!(feeds[0].stalls.is_empty());

    // The same month stalled on both rungs is two entries, one each.
    let mut both = groww(month(2022, 2));
    both.enter(DAY);
    stall_out(&mut both);
    both.enter(MINUTE);
    both.frontier = month(2022, 2);
    stall_out(&mut both);
    stall_out(&mut both);
    assert_eq!(both.stalls.len(), 3, "{:?}", both.stalls);
    let json = Status {
        feeds: vec![FeedReport {
            stalls: both.stalls.clone(),
            ..FeedReport::default()
        }],
        ..Status::default()
    }
    .json();
    assert!(json.contains(r#""timeframe":"1day""#), "{json}");
    assert!(json.contains(r#""timeframe":"1min""#), "{json}");
}

// ------------------------------------------------------------ W1-api1-1

/// **W1-api1-1. The per-rung work list is built once per masters parse, not
/// once per pass.**
///
/// `fly` called `tracked_series` on every pass — a filter, sort and dedup of
/// the whole merged universe. This counts the builds: a thousand passes over
/// both rungs cost two, a reparse costs two more, and the lists are exactly
/// what `tracked_series` answers.
#[test]
fn the_rung_work_lists_are_built_once_per_masters_parse() {
    let dir = crate::scratch::path("autopilot-series-cache");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let site = Loaded::new(Site::load(&dir, &dir));
    let rungs: Vec<Timeframe> = RUNGS.iter().filter_map(|r| r.store_timeframe()).collect();
    assert_eq!(rungs.len(), 2);
    let generation = site.universe().generation;

    let mut cache = SeriesCache::default();
    assert_eq!(cache.builds, 0);
    for pass in 0..1_000usize {
        let tf = rungs[pass % 2];
        let got = cache.get(&site, tf).to_vec();
        assert_eq!(got, tracked_series(&site, tf));
    }
    assert_eq!(cache.builds, 2, "one build per rung, not one per pass");

    site.reparse(&dir)
        .expect("an empty masters dir reparses over an empty universe");
    assert_eq!(
        site.universe().generation,
        generation.wrapping_add(1),
        "every swap moves the generation"
    );
    for pass in 0..1_000usize {
        cache.get(&site, rungs[pass % 2]);
    }
    assert_eq!(cache.builds, 4, "a reparse costs one rebuild per rung");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **The slots fill in order, and a third timeframe evicts the FIRST slot.**
///
/// `SeriesCache::get` documents that slots fill in order and that a third
/// timeframe reuses the first slot. Asked `DAY_1`, then `MINUTE_1`, then `MINUTE_5`:
/// `DAY_1` sits in the first slot and is the one evicted, so `MINUTE_1` is still
/// held in the second (no rebuild) and `DAY_1` costs one. Four builds in all; a
/// cache that filled the second slot first would evict `MINUTE_1` instead and
/// pay five.
#[test]
fn a_third_timeframe_evicts_the_first_filled_slot_not_the_second() {
    let dir = crate::scratch::path("autopilot-series-cache-third");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let site = Loaded::new(Site::load(&dir, &dir));
    let mut cache = SeriesCache::default();
    for tf in [Timeframe::DAY_1, Timeframe::MINUTE_1, Timeframe::MINUTE_5] {
        cache.get(&site, tf);
    }
    assert_eq!(cache.builds, 3, "three distinct timeframes, three builds");
    cache.get(&site, Timeframe::MINUTE_1);
    assert_eq!(cache.builds, 3, "MINUTE_1 survived in the second slot");
    cache.get(&site, Timeframe::DAY_1);
    assert_eq!(cache.builds, 4, "DAY_1 was the evicted first slot");
    let _ = std::fs::remove_dir_all(&dir);
}
