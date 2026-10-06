#![cfg(test)]
//! Gate 18 run 1283, group cli-a: one test per surviving `lib.rs` mutant, each
//! asserting the exact behaviour the mutant changed (G18-cli-a-NN).
#![expect(clippy::expect_used, reason = "a failed fixture must fail its test")]

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A private scratch directory, removed on drop.
struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "brutex-g18-cli-a-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("scratch root");
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const fn candle(minute: i64, close: i64) -> indicators::Candle {
    indicators::Candle {
        ts_micros: 1_746_000_000_000_000 + minute * 60_000_000,
        open: close,
        high: close + 100,
        low: close - 100,
        close,
        volume: 0,
        open_interest: indicators::OI_NULL,
    }
}

fn run_id() -> runner::identity::RunId {
    let key = crate::stored::swept_index("NIFTY").expect("fixture key");
    runner::identity::identity(&runner::identity::Run {
        mask: vocab::ConditionMask::default(),
        direction: runner::identity::Direction::Undirected,
        instrument: &key,
        timeframe: "1min",
        params: runner::identity::Params::of(engine::Ladder::with_min_hits(1).with_ceiling(64)),
        data_digest: runner::identity::data_digest(&[candle(0, 2_500_000)]),
        commit: "generated-g18-cli-a-fixture",
        feed: "zerodha",
    })
}

/// G18-cli-a-21: a refusal names an identity as 64 lowercase hex digits.
#[test]
fn hex_of_spells_every_byte_as_two_lowercase_digits() {
    let mut identity = [0xab_u8; 32];
    identity[0] = 0x01;
    identity[31] = 0xf0;
    let hex = hex_of(&identity);
    assert_eq!(hex.len(), 64);
    assert!(hex.starts_with("01abab"), "{hex}");
    assert!(hex.ends_with("abf0"), "{hex}");
}

/// G18-cli-a-22: understood work exits `OK` unless its page carries a refusal.
#[test]
fn work_exit_is_ok_for_an_answer_and_failed_for_a_refusal() {
    assert_eq!(work_exit("TOP 10 BY RUNG\n  60s\n"), OK);
    assert_eq!(work_exit("refused: the store is empty\n"), FAILED);
}

/// G18-cli-a-23: both ledger arms refuse words they do not understand as
/// `MISUSED`, with the reason and the usage.
#[test]
fn the_ledger_arms_exit_misused_on_words_they_do_not_understand() {
    for v6 in [false, true] {
        let mut out = String::new();
        let code = ledger_all_arm(
            &mut out,
            "zerodha",
            ("2025", "1"),
            ("2025", "3"),
            ("1000", "0"),
            "/nonexistent",
            v6,
        );
        assert_eq!(code, MISUSED, "{out}");
        assert!(out.contains("MAX_POINTS must be a whole number"), "{out}");
        assert!(out.contains(USAGE), "{out}");
    }
    let mut out = String::new();
    assert_eq!(ledger_replay_arm(&mut out, &[]), MISUSED, "{out}");
    assert!(out.contains("requires training and explicit OOS months"), "{out}");
    assert!(out.contains(USAGE), "{out}");
}

/// G18-cli-a-24: the stamped preparation never answers without a store.
/// Unstamped, it refuses for the stamp; stamped, the missing root refuses.
#[test]
fn a_stored_preparation_over_a_missing_root_refuses() {
    let folded = vec![candle(0, 2_500_000)];
    let mut days = Vec::new();
    let mut bars = folded.clone();
    let refused = column_withholding_unsourceable_days(
        std::path::Path::new("/nonexistent/brutex-g18-store"),
        parse_vendor("zerodha").expect("feed"),
        "NIFTY",
        ((2025, 5), (2025, 5)),
        FoldedSeries {
            folded: &folded,
            days: &mut days,
            bars: &mut bars,
        },
        60_000_000,
        "1min",
    );
    let why = refused.err().expect("no store, no column");
    if commit_stamp().is_none() {
        assert!(why.contains("no verified commit stamp"), "{why}");
    }
}

/// G18-cli-a-25: the anchored digest is a function of the signal: equal
/// inputs give equal bytes, a moved close gives different bytes.
#[test]
fn the_anchored_digest_moves_with_the_signal() {
    let exact = stored::ExactMinuteContext {
        bars: Vec::new(),
        asked: 1,
        found: 0,
        prior_session_day: 0,
        prior_session_bars: 0,
        excluded: stored::CalendarExclusion::none(),
        cash: None,
    };
    let daily = stored::DailyContext {
        bars: Vec::new(),
        references: Vec::new(),
        eligibility: Vec::new(),
        asked: 1,
        found: 0,
    };
    let one = [candle(0, 2_500_000), candle(1, 2_500_100)];
    let other = [candle(0, 2_500_000), candle(1, 2_500_200)];
    let first = stored_anchored_digest(&one, &exact, &daily).expect("digest");
    assert_eq!(
        stored_anchored_digest(&one, &exact, &daily).expect("digest"),
        first
    );
    assert_ne!(
        stored_anchored_digest(&other, &exact, &daily).expect("digest"),
        first
    );
}

/// G18-cli-a-26: the requested span is appended as `year * 100 + month`.
#[test]
fn span_policy_appends_the_span_as_year_month_numbers() {
    let mut policy = [0_u64; 21];
    for (term, slot) in (7_u64..).zip(policy.iter_mut()) {
        *slot = term;
    }
    let out = span_policy(policy, (2025, 1), (2026, 12));
    assert_eq!(out[..21], policy[..]);
    assert_eq!(out[21], 202_501);
    assert_eq!(out[22], 202_612);
}

/// G18-cli-a-27: a validation re-run that meets no rule hands back the
/// search page it was given, byte for byte.
#[test]
fn validated_at_returns_the_search_page_when_nothing_was_met() {
    let question = AttemptRung {
        vendor_word: "zerodha",
        underlying: "NOT-A-SWEPT-INDEX",
        rung: "5min",
        span: ((2025, 5), (2025, 5)),
        bars: 0,
        attempt: None,
    };
    let policy = Policy {
        rules: Rules::operator(),
        lens: runner::rank::Lens::Payoff,
        validate: false,
    };
    let searched = "SEARCHED PAGE, step 3 of 10\n";
    let page = validated_at(
        question,
        1_000,
        policy,
        searched,
        &mut ScreenCache::default(),
    );
    assert_eq!(page, searched);
}

/// G18-cli-a-28: `descend` answers with the refusal of the rung it does not
/// sweep, never an empty page.
#[test]
fn descend_refuses_an_unknown_rung_by_name() {
    let page = descend(
        "zerodha",
        "NIFTY",
        "7min",
        (2025, 5),
        (2025, 5),
        0,
        Cadence::PerWeek(1),
    );
    assert!(
        page.starts_with("refused: `7min` is not a rung this engine sweeps"),
        "{page}"
    );
}

/// G18-cli-a-29: trades are written once and counted, then reused by the
/// exact rerun with the same count.
#[test]
fn record_trades_reports_and_counts_the_rows_it_wrote_then_reuses() {
    let scratch = Scratch::new("trades");
    let id = run_id();
    let row = grid::TradeRow {
        signal_bar: 1,
        entry_bar: 2,
        exit_bar: 4,
        best: 400,
        worst: 100,
        entry_micros: 1_746_000_120_000_000,
        exit_micros: 1_746_000_240_000_000,
        adverse: 1_000,
        adverse_paisa: 20,
        favourable: 2_000,
        favourable_paisa: 40,
    };
    let chosen = [row, row];
    let (report, count) =
        record_trades(&scratch.0, &id, Direction::Long, &chosen).expect("written");
    assert_eq!(count, 2);
    assert!(
        report.contains("2 trade(s) prepared and synced"),
        "{report}"
    );
    let (again, count) =
        record_trades(&scratch.0, &id, Direction::Long, &chosen).expect("reused");
    assert_eq!(count, 2);
    assert!(again.contains("2 trade(s) already present"), "{again}");
}

/// G18-cli-a-30: the live view opens with no refusal, and its bar is the
/// ceiling of the Bonferroni `|t|` in thousandths.
#[test]
fn publish_ranked_opens_the_live_view_with_the_bar_in_thousandths() {
    let scratch = Scratch::new("live");
    let id = run_id();
    let outcome = runner::RankedOutcome {
        census: indicators::column::Census::default(),
        first_swept: Some(0),
        trials: 15,
        effective_trials: 14,
        closure_complete: true,
        sweep: engine::keep::Streamed::default(),
    };
    let (live, refusal) = publish_ranked(&scratch.0, &id, &[], &outcome, Rules::operator());
    assert_eq!(refusal, "");
    assert!(live.is_some(), "the live view was started");
    let published = crate::live::current(&scratch.0);
    let summary = published
        .iter()
        .find(|(identity, _, _)| *identity == id.bytes())
        .map(|(_, summary, _)| *summary)
        .expect("this run's live view");
    assert_eq!(summary.trials, 14);
    assert_eq!(summary.priced, 0);
    // The bar a reader compares `t_milli` against: at 14 trials the
    // Bonferroni `|t|` is between 2 and 4, so a thousandths figure is between
    // 2,000 and 4,000; added (not multiplied) or divided it is near 1,000 or 1.
    assert!(
        (2_000..4_000).contains(&summary.bar_milli),
        "{}",
        summary.bar_milli
    );
    drop(live);
}

/// G18-cli-a-31: every tick is counted, and a progress line lands in the log.
#[test]
fn grid_progress_counts_each_tick_and_notes_progress() {
    let progress = GridProgress::over(3, None);
    progress.tick();
    progress.tick();
    assert_eq!(progress.counted.load(Ordering::Relaxed), 2);

    let sink = crate::ledger_all::tests::sink();
    let from = crate::ledger_all::tests::mark();
    note_grid_progress(None, 918_273_645, 918_273_646);
    let dir = sink
        .path()
        .parent()
        .expect("the sink writes its file inside a directory")
        .to_path_buf();
    let query = telemetry::Query::last(telemetry::MAX_LIMIT).from_target("cli.audit");
    let landed = telemetry::tail(&dir, sink.keep_files(), &query)
        .records
        .into_iter()
        .filter(|record| {
            record.seq >= from
                && record.message == "exit grid progress"
                && crate::ledger_all::tests::counts(record, "priced", 918_273_645)
        })
        .count();
    assert_eq!(landed, 1);
}

/// G18-cli-a-13: the forced stop is the stated ceiling, and none at zero.
#[test]
fn the_forced_stop_is_the_ceiling_and_none_without_one() {
    let with = |max_mae_ppm| Rules {
        max_mae_ppm,
        ..Rules::operator()
    };
    assert_eq!(with(0).forced_stop(), None);
    assert_eq!(with(-5).forced_stop(), None);
    assert_eq!(with(1).forced_stop(), Some(1));
    assert_eq!(with(30_000).forced_stop(), Some(30_000));
}

/// G18-cli-a-15: half rounds away from zero, on both sides of it.
#[test]
fn half_rounds_away_from_zero_on_both_signs() {
    assert_eq!(div_round_half_away(5, 2), 3);
    assert_eq!(div_round_half_away(-5, 2), -3);
    assert_eq!(div_round_half_away(7, 4), 2);
    assert_eq!(div_round_half_away(-7, 4), -2);
    assert_eq!(div_round_half_away(-1, 3), 0);
    assert_eq!(div_round_half_away(0, 3), 0);
}

fn traded(trades: u64, pessimistic: i64) -> grid::Cell {
    grid::Cell {
        trades,
        wins: trades,
        pessimistic,
        optimistic: pessimistic,
        ..grid::Cell::default()
    }
}

/// G18-cli-a-12: a side whose shown cell never traded is not a row.
#[test]
fn a_shown_cell_that_never_traded_is_not_a_row() {
    let idle = traded(0, 0);
    assert_eq!(best_shown([("long", Some((idle, true))), ("short", None)]), None);
    let busy = traded(3, 10);
    assert_eq!(
        best_shown([("long", Some((idle, true))), ("short", Some((busy, false)))]),
        Some(("short", busy, false))
    );
}

/// G18-cli-a-32: pruning keeps admitted cells that TRADED, never a cell
/// that did not, however permissive the envelope.
#[test]
fn pruning_drops_a_cell_that_never_traded() {
    let envelope = Rules {
        max_mae_ppm: 0,
        min_rr_bp: 0,
        min_win_rate_bp: 0,
        min_assurance_bp: 0,
        min_weakest_bp: 0,
        min_trades: 0,
        min_ret_over_dd_bp: 0,
        min_fill_headroom_bp: 0,
        min_avg_rr_bp: 0,
        require_protective_exits: false,
        top: 10,
    };
    let idle = traded(0, 0);
    let busy = traded(3, 10);
    assert!(envelope.admits(&idle), "premise: the envelope admits it");
    let full = grid::Grid {
        cells: vec![idle, busy],
        ..grid::Grid::default()
    };
    assert_eq!(prune_cells(full, envelope).cells, vec![busy]);
}

/// G18-cli-a-20: zero is no ceiling and reads no reference; a ceiling
/// converts against the reference, and one converting to nothing refuses.
#[test]
fn elite_ceiling_is_zero_without_a_reference_and_converted_with_one() {
    assert_eq!(
        elite_ceiling_ppm(0, || Err("the reference must not be read".to_owned())),
        Ok(0)
    );
    assert_eq!(
        elite_ceiling_ppm(5, || Ok(2_500_000)),
        Ok(points_to_ppm_at(5, 2_500_000))
    );
    assert_eq!(points_to_ppm_at(5, 2_500_000), 200);
    assert_eq!(
        elite_ceiling_ppm(5, || Err("span refused".to_owned())),
        Err("span refused".to_owned())
    );
    let nothing = elite_ceiling_ppm(1, || Ok(200_000_000)).expect_err("zero ppm");
    assert!(nothing.contains("admits nothing"), "{nothing}");
}

/// G18-cli-a-33: only a NEGATIVE ceiling or an out-of-range TOP is refused
/// by the points door; zero and a positive ceiling pass it.
#[test]
fn screen_in_points_refuses_only_a_negative_ceiling_or_a_bad_top() {
    const REFUSAL: &str = "the stop ceiling is a whole number of index points";
    let screen = |max_points, top| {
        screen_range_in_points(
            "zerodha",
            "NOT-A-SWEPT-INDEX",
            "5min",
            ((2025, 5), (2025, 5)),
            1_000,
            max_points,
            top,
        )
    };
    assert!(screen(-1, 10).contains(REFUSAL));
    assert!(screen(5, 0).contains(REFUSAL));
    for (max_points, top) in [(0, 10), (5, 10), (5, 1)] {
        let page = screen(max_points, top);
        assert!(!page.contains(REFUSAL), "{max_points} {top}: {page}");
    }
}
