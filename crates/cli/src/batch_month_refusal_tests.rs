#![cfg(test)]
//! `sweep-all` refuses a month whose column swept no bar, with the sentence
//! `sweep-stored` uses, and names `CEILING` only for a candidate-ceiling halt
//! (g-zerodha-universe-census-2).
//!
//! Generated stores only: invented constant-price sessions at paisa values no
//! real index printed, written through the store's own writer.
#![allow(clippy::expect_used)]

use super::*;
use crate::results::Results;
use crate::sweep_evidence::{self, Completion};
use brutex_core::vendor::Vendor;
use std::path::Path;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

const COMMIT: &str = "generated-batch-month-refusal-fixture";

/// One open session of constant invented minutes, or nothing on a closed day.
fn session(month: u8, date: u8) -> Vec<Bar> {
    let civil = pull::session::Day::new(2025, month, date).expect("date");
    let day = i64::from(civil.days_from_epoch());
    let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
        return Vec::new();
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| window.from..=window.to)
        .map(|minute| Bar {
            ts_micros: day * 86_400_000_000 + i64::from(minute) * 60_000_000
                - indicators::IST_OFFSET_MICROS,
            open: 100_000,
            high: 110_000,
            low: 90_000,
            close: 101_000,
            volume: 100,
            open_interest: i64::MIN,
        })
        .collect()
}

fn write(root: &Path, symbol: &str, month: u8, timeframe: Timeframe, rows: &[Bar]) {
    let key = crate::stored::swept_index(symbol).expect("key");
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        timeframe,
        YearMonth::new(2025, month).expect("month"),
        FileKind::Bars,
    )
    .expect("path");
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    let id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    BarFile::open_or_create(root, path, id)
        .expect("writer")
        .append(rows)
        .expect("generated rows");
}

/// Two seed sessions of `symbol` (30 April and 2 May 2025): enough to load,
/// too few 5min bars (75) to warm the column, as `auto-stored`'s seeded
/// fixture measures.
fn seed_unwarmable(root: &Path, symbol: &str) {
    for (month, date) in [(4, 30), (5, 2)] {
        let rows = session(month, date);
        assert!(
            !rows.is_empty(),
            "premise: 2025-{month:02}-{date:02} is a session"
        );
        write(root, symbol, month, Timeframe::MINUTE_1, &rows);
        write(
            root,
            symbol,
            month,
            Timeframe::DAY_1,
            std::slice::from_ref(rows.first().expect("a session row")),
        );
        if month == 5 {
            let five: Vec<Bar> = rows.iter().step_by(5).copied().collect();
            write(root, symbol, month, Timeframe::MINUTE_5, &five);
        }
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-batch-month-refusal-{name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("scratch root");
    root
}

/// A MONTH WHOSE COLUMN SWEPT NO BAR IS REFUSED, NOT A CEILING STOP.
///
/// Beside a warmed NIFTY month that sweeps, BANKNIFTY's 5min month holds 75
/// bars and warms nothing. Before the fix its row read `0 bars k=0 0 kept
/// CEILING — depth not reached by extinction`, a ledger row was filed and the
/// summary said one month "stopped on the candidate ceiling". Now it is
/// `REFUSED` with `sweep-stored`'s own sentence, files no ledger row, finishes
/// its attempt `Refused`, and is counted as refused.
#[test]
fn a_sweep_all_month_that_swept_no_bar_is_refused_not_ceiling() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let text = crate::audited_stored::with_warmed_store(|root| {
        seed_unwarmable(root, "BANKNIFTY");
        let text = sweep_under(root, "zerodha", "5min", u64::MAX, COMMIT)
            .expect("one month swept, so the run is a report");
        let mut ledger = Results::open_read(root).expect("ledger");
        assert_eq!(
            ledger.len().expect("rows"),
            1,
            "only the month that swept is filed:\n{text}"
        );
        let row = ledger.read(0).expect("the filed row");
        assert_eq!(crate::results::read_field(&row.underlying), "NIFTY");
        text
    });
    crate::knobs::clear_all();
    assert!(
        text.contains(&format!(
            "REFUSED  zerodha BANKNIFTY 5min 2025-05  — {}",
            crate::and_checkpoint::UNMEASURED_COLUMN
        )),
        "{text}"
    );
    assert!(text.contains("none swept past the warm-up"), "{text}");
    assert!(!text.contains("CEILING"), "{text}");
    assert!(!text.contains("stopped on the candidate ceiling"), "{text}");
    assert!(text.contains("1 swept · 1 refused"), "{text}");
}

/// A STORE OF ONLY SUCH MONTHS IS REFUSED AS A WHOLE (D-0696), so the arm
/// exits `FAILED`; the month's attempt is finished `Refused`, never filed.
#[test]
fn a_store_of_only_unwarmable_months_is_refused() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let root = scratch("only");
    seed_unwarmable(&root, "NIFTY");
    let why =
        sweep_under(&root, "zerodha", "5min", 1, COMMIT).expect_err("every month offered refused");
    crate::knobs::clear_all();
    assert!(
        why.contains(crate::and_checkpoint::UNMEASURED_COLUMN),
        "{why}"
    );
    assert!(why.contains("none of the 1 month(s) offered"), "{why}");
    assert!(!why.contains("CEILING"), "{why}");
    assert_eq!(
        crate::work_exit(&format!("refused: {why}\n")),
        crate::FAILED
    );
    assert_eq!(
        Results::open_read(&root).map_or(0, |ledger| ledger.len().expect("rows")),
        0,
        "no ledger row is filed for a month that swept nothing"
    );
    let evidence = sweep_evidence::latest(&root, 1_048_576)
        .expect("evidence reads")
        .expect("the month's attempt was begun");
    assert_eq!(evidence.completion, Completion::Refused);
}

fn row(completed: bool, halt: Option<engine::Breach>) -> Row {
    Row {
        label: "zerodha NIFTY 1min 2025-05".to_owned(),
        bars: 10,
        depth: 1,
        kept: 0,
        completed,
        halt,
        identity: None,
        refused: None,
        ran: true,
    }
}

/// `CEILING` names a candidate-ceiling halt and nothing else; every other
/// incomplete walk names its own cause, and only the ceiling halt is counted
/// as "stopped on the candidate ceiling".
#[test]
fn ceiling_is_named_only_for_a_ceiling_halt() {
    use engine::Breach;
    assert_eq!(stop_label(&row(true, None)), "");
    assert!(stop_label(&row(false, Some(Breach::Candidates))).contains("CEILING"));
    for other in [
        Some(Breach::Pairs),
        Some(Breach::Memory),
        Some(Breach::Workers),
        None,
    ] {
        let label = stop_label(&row(false, other));
        assert!(!label.is_empty(), "{other:?}");
        assert!(!label.contains("CEILING"), "{other:?}: {label}");
    }
    let mut tally = Tally::default();
    for r in [
        row(false, Some(Breach::Candidates)),
        row(false, Some(Breach::Pairs)),
        row(false, None),
        row(true, None),
    ] {
        tally.fold(&r);
    }
    assert_eq!(
        (tally.swept, tally.incomplete, tally.stopped_short),
        (4, 1, 2)
    );
    let rows = [row(false, Some(Breach::Pairs))];
    let only_short = Tally {
        offered: 1,
        swept: 1,
        stopped_short: 1,
        ..Tally::default()
    };
    let text = render(
        "zerodha",
        "1min",
        1,
        COMMIT,
        "",
        &store::catalog::Census::default(),
        &only_short,
        &rows,
    );
    assert!(!text.contains("stopped on the candidate ceiling"), "{text}");
    assert!(!text.contains("CEILING"), "{text}");
    assert!(text.contains("HALTED on the pair budget"), "{text}");
    assert!(text.contains("1 month(s) stopped short"), "{text}");
}
