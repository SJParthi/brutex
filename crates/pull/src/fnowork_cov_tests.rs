//! BASE-COV-PULL: the futures plan's and pull's refusals no test had reached
//! (Route B, recipe v4). Every row is invented: the underlying `ZQIDX`, tokens
//! in the eight millions, as in `fnowork_futures_tests`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::cell::RefCell;

use super::futures_tests::{
    Vendorish, block_on, day, fut, route, scratch, snapshot_on, spec, window,
};
use super::*;
use crate::chain::Refusal;
use crate::master_snapshots::{self, Book, Kind as Master, Stamp};
use crate::vendor::{FnoAccess, Granularity, HttpSpec};
use brutex_core::vendor::Vendor;

/// A future row of `name`, for a token that once named `ZQIDX`'s.
fn fut_of(token: u64, name: &str, expiry: &str) -> String {
    format!(
        "{token},{},{name}26JANFUT,\"{name}\",0,{expiry},0,0.05,75,FUT,NFO-FUT,NFO",
        token / 256
    )
}

fn plan_of(book: &Book, asked: Window, today: Day, settled: Day) -> FuturesPlan {
    futures_plan(book, route(), "ZQIDX", asked, today, settled).unwrap()
}

/// `n` entries of a body cut before its receipt, each a stamp of one day.
fn cut_entries(masters: &std::path::Path, n: usize) {
    for ordinal in 1..=n {
        let stamp = Stamp {
            day: day(2026, 1, 31),
            ordinal: u32::try_from(ordinal).unwrap(),
        };
        let name = master_snapshots::file_name(Master::Kite, stamp);
        std::fs::write(masters.join(name), b"cut before its receipt").unwrap();
    }
}

/// At the bound every cut entry is named and no line counts the rest; one
/// past it, the bound's newest are named and one line counts the rest.
#[test]
fn a_book_of_more_cut_entries_than_a_plan_names_counts_the_rest() {
    let bound = master_snapshots::INCOMPLETE_NAMED;
    let masters = scratch("cut-at-bound");
    snapshot_on(
        &masters,
        day(2026, 1, 5),
        &[fut(8_000_011, "ZQIDX26JANFUT", "2026-01-27")],
    );
    cut_entries(&masters, bound);
    let lines = incomplete_lines(&Book::open(&masters).unwrap());
    assert_eq!(lines.len(), bound);
    assert!(
        lines.iter().all(|line| line.contains("never read")),
        "{lines:?}"
    );
    cut_entries(&masters, bound + 1);
    let lines = incomplete_lines(&Book::open(&masters).unwrap());
    assert_eq!(lines.len(), bound + 1);
    assert_eq!(
        lines[bound],
        "1 older incomplete entries are held and not named here"
    );
}

/// A settled day before the window: no contract's span and no continuous span
/// is left, and that is an empty plan, not a refusal (a combination of two
/// extremes: a contract wholly before the window, and a window wholly past
/// what has settled).
#[test]
fn a_window_past_what_has_settled_plans_nothing_and_refuses_nothing() {
    let masters = scratch("past-settled");
    snapshot_on(
        &masters,
        day(2026, 1, 5),
        &[
            fut(8_000_011, "ZQIDX26JANFUT", "2026-01-27"),
            fut(8_000_012, "ZQIDX26MARFUT", "2026-03-31"),
        ],
    );
    let book = Book::open(&masters).unwrap();
    let asked = window(day(2026, 2, 2), day(2026, 2, 6));
    let plan = plan_of(&book, asked, day(2026, 2, 1), day(2026, 1, 30));
    assert_eq!(plan.asks, []);
    assert_eq!(plan.refused, Vec::<String>::new());
}

/// A contract the newest snapshot no longer lists is still asked: its token's
/// history stands on the snapshots that did list it.
#[test]
fn a_contract_gone_from_the_newest_snapshot_is_still_asked() {
    let masters = scratch("not-listed");
    let (jan, feb) = (
        fut(8_000_011, "ZQIDX26JANFUT", "2026-01-27"),
        fut(8_000_012, "ZQIDX26FEBFUT", "2026-02-24"),
    );
    snapshot_on(&masters, day(2026, 1, 5), &[jan, feb.clone()]);
    snapshot_on(&masters, day(2026, 1, 30), &[feb]);
    let book = Book::open(&masters).unwrap();
    let asked = window(day(2026, 1, 5), day(2026, 1, 27));
    let plan = plan_of(&book, asked, day(2026, 1, 30), day(2026, 1, 27));
    assert_eq!(plan.refused, Vec::<String>::new());
    let asked = plan.asks.iter().filter(|ask| ask.token == 8_000_011);
    assert_eq!(asked.count(), 1, "{plan:?}");
}

/// A token the newest snapshot gives to another underlying is refused by
/// name, and so is one it carries twice, which no snapshot should.
#[test]
fn a_token_reused_by_another_share_or_carried_twice_is_refused_by_name() {
    let masters = scratch("reused-share");
    let jan = fut(8_000_011, "ZQIDX26JANFUT", "2026-01-27");
    let feb = fut(8_000_012, "ZQIDX26FEBFUT", "2026-02-24");
    snapshot_on(&masters, day(2026, 1, 5), &[jan.clone(), feb.clone()]);
    snapshot_on(
        &masters,
        day(2026, 1, 8),
        &[fut_of(8_000_011, "ZQOTHER", "2026-01-27"), feb.clone()],
    );
    let book = Book::open(&masters).unwrap();
    let asked = window(day(2026, 1, 5), day(2026, 1, 7));
    let plan = plan_of(&book, asked, day(2026, 1, 8), day(2026, 1, 7));
    assert_eq!(plan.refused.len(), 1, "{:?}", plan.refused);
    assert!(
        plan.refused[0].starts_with("token_reused: "),
        "{:?}",
        plan.refused
    );
    assert!(plan.refused[0].contains("ZQOTHER"), "{:?}", plan.refused);

    let twice = scratch("carried-twice");
    snapshot_on(&twice, day(2026, 1, 5), &[jan.clone(), feb.clone()]);
    snapshot_on(&twice, day(2026, 1, 8), &[jan.clone(), jan, feb]);
    let book = Book::open(&twice).unwrap();
    let plan = plan_of(&book, asked, day(2026, 1, 8), day(2026, 1, 7));
    let named = plan.refused.iter();
    let twice = named.filter(|why| why.starts_with("token_duplicated: "));
    assert_eq!(twice.count(), 1, "{:?}", plan.refused);
}

#[test]
fn a_continuous_ask_with_no_snapshot_is_refused_by_name() {
    let book = Book::open(&scratch("no-snapshot")).unwrap();
    let on = day(2026, 1, 5);
    let why = continuous_ask(&book, route(), "ZQIDX", on, on, on).unwrap_err();
    assert_eq!(why, "no_snapshot: no dated Kite master snapshot is held");
}

fn one_ask(series: FuturesSeries, at: Window) -> FuturesPlan {
    FuturesPlan {
        underlying: "ZQIDX".to_owned(),
        newest: Stamp {
            day: day(2026, 1, 5),
            ordinal: 1,
        },
        asks: vec![FuturesAsk {
            token: 8_000_011,
            series,
            window: at,
        }],
        refused: Vec::new(),
        rebuilt: None,
        incomplete: Vec::new(),
    }
}

/// `plan` pulled against a vendor answering `reply`; what it asked and landed.
fn pull(
    from_spec: &HttpSpec,
    plan: &FuturesPlan,
    reply: fn(&str) -> Result<String, Refusal>,
) -> (Vec<String>, FuturesLanded) {
    let vendor = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply,
    };
    let landed = block_on(pull_futures_day(
        &vendor,
        from_spec,
        plan,
        &scratch("pull-store"),
        Vendor::Zerodha,
        "test://futures",
    ));
    (vendor.seen.into_inner(), landed)
}

fn never(url: &str) -> Result<String, Refusal> {
    panic!("nothing should be asked, and {url} was");
}

fn the_stop(landed: &FuturesLanded) -> String {
    format!("{landed:?}")
}

#[test]
fn a_feed_with_no_token_route_asks_nothing_and_says_so() {
    let mut no_route = spec();
    no_route.fno = FnoAccess::None;
    let at = window(day(2026, 1, 5), day(2026, 1, 9));
    let (seen, landed) = pull(&no_route, &one_ask(FuturesSeries::Continuous, at), never);
    assert_eq!(seen, Vec::<String>::new());
    assert_eq!(
        landed.failed,
        ["this feed has no futures token route; nothing was asked"]
    );
}

#[test]
fn a_continuous_symbol_that_does_not_file_stops_its_series_before_any_ask() {
    let at = window(day(2026, 1, 5), day(2026, 1, 9));
    let mut plan = one_ask(FuturesSeries::Continuous, at);
    plan.underlying = "NOT A SYMBOL".to_owned();
    let (seen, landed) = pull(&spec(), &plan, never);
    assert_eq!(seen, Vec::<String>::new());
    assert!(
        the_stop(&landed).contains("the continuous symbol does not file"),
        "{landed:?}"
    );
}

#[test]
fn a_cap_of_zero_days_stops_its_series_before_any_ask() {
    static NO_DAYS: [(Granularity, u32); 1] = [(Granularity::Day1, 0)];
    let mut capped = spec();
    capped.window_caps = &NO_DAYS;
    let at = window(day(2026, 1, 5), day(2026, 1, 9));
    let (seen, landed) = pull(&capped, &one_ask(FuturesSeries::Continuous, at), never);
    assert_eq!(seen, Vec::<String>::new());
    assert!(
        the_stop(&landed).contains("the window does not split to the cap"),
        "{landed:?}"
    );
}

#[test]
fn a_rung_the_feed_cannot_spell_stops_its_series_before_any_ask() {
    let mut mute = spec();
    mute.granularity_tokens = &[];
    let at = window(day(2026, 1, 5), day(2026, 1, 9));
    let (seen, landed) = pull(&mute, &one_ask(FuturesSeries::Continuous, at), never);
    assert_eq!(seen, Vec::<String>::new());
    assert!(the_stop(&landed).contains("cannot spell"), "{landed:?}");
}

#[test]
fn an_answer_that_does_not_decode_stops_its_series_by_name() {
    let at = window(day(2026, 1, 5), day(2026, 1, 9));
    let (seen, landed) = pull(&spec(), &one_ask(FuturesSeries::Continuous, at), |_| {
        Ok("this is not the vendor's shape".to_owned())
    });
    assert_eq!(seen.len(), 1);
    assert!(the_stop(&landed).contains("does not decode"), "{landed:?}");
}

/// "Already held" is claimed only of a series that did not stop.
#[test]
fn a_stopped_series_claims_nothing_already_held() {
    let done = SeriesOutcome {
        offered: 5,
        committed: 2,
        ..SeriesOutcome::default()
    };
    assert!(done.line("L", "W").contains("(3 already held)"));
    let stopped = done.stop("the vendor went quiet".to_owned());
    let line = stopped.line("L", "W");
    assert!(!line.contains("already held"), "{line}");
    assert!(
        line.ends_with("; then stopped: the vendor went quiet"),
        "{line}"
    );
    let nothing = SeriesOutcome::default().line("L", "W");
    assert!(!nothing.contains("already held"), "{nothing}");
}

/// A cell held to the last day the `Day` type can state has no day after it
/// to ask for: complete, never asked again and never a panic.
#[test]
fn a_cell_held_to_the_last_representable_day_is_complete() {
    let last = Day::new(9999, 12, 31).unwrap();
    assert!(last.succ().is_err(), "the premise of this test");
    assert_eq!(the_day_after(last, last), None);
    assert_eq!(the_day_after(day(2026, 1, 5), day(2026, 1, 5)), None);
    assert_eq!(the_day_after(day(2026, 1, 6), day(2026, 1, 5)), None);
    assert_eq!(
        the_day_after(day(2026, 1, 5), day(2026, 1, 6)),
        Some(day(2026, 1, 6))
    );
}
