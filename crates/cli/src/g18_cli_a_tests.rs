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
    assert_eq!(hex_of(&[0; 32]), "0".repeat(64));
    assert_eq!(hex_of(&[0xff; 32]), "f".repeat(64));
}

/// G18-cli-a-22: understood work exits `OK` unless its page carries a refusal.
#[test]
fn work_exit_is_ok_for_an_answer_and_failed_for_a_refusal() {
    assert_eq!(work_exit("TOP 10 BY RUNG\n  60s\n"), OK);
    assert_eq!(work_exit("refused: the store is empty\n"), FAILED);
    assert_eq!(work_exit(""), OK);
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
    assert!(
        out.contains("requires training and explicit OOS months"),
        "{out}"
    );
    assert!(out.contains(USAGE), "{out}");
}

/// G18-cli-a-24: the stamped preparation never answers without a store.
///
/// Its subject was `column_withholding_unsourceable_days`, the stamping
/// wrapper `one_rung`'s derived support alone called. D-4719 removed both the
/// call and the wrapper, so the stamped build itself is asked: with an
/// admitted build stamped, a missing root refuses, and nothing is built.
#[test]
fn a_stored_preparation_over_a_missing_root_refuses() {
    let folded = vec![candle(0, 2_500_000)];
    let mut days = Vec::new();
    let mut bars = folded.clone();
    let root = std::path::Path::new("/nonexistent/brutex-g18-store");
    let refused = column_withholding_at_build(
        root,
        parse_vendor("zerodha").expect("feed"),
        "NIFTY",
        ((2025, 5), (2025, 5)),
        FoldedSeries {
            folded: &folded,
            days: &mut days,
            bars: &mut bars,
        },
        60_000_000,
        StoredPreparationBuild {
            rung: "1min",
            commit: Some("generated-g18-missing-root"),
        },
    );
    assert!(refused.is_err(), "no store, no column");
    assert!(
        days.is_empty(),
        "no day was withheld from a store never read"
    );
    assert!(!root.exists(), "and none was created");
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
    // The extremes of the two words: year zero, month one, and the widest year.
    let edge = span_policy([0; 21], (0, 1), (u16::MAX, 12));
    assert_eq!(edge[21], 1);
    assert_eq!(edge[22], 6_553_512);
    assert_eq!(edge[..21], [0_u64; 21]);
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
    let (again, count) = record_trades(&scratch.0, &id, Direction::Long, &chosen).expect("reused");
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
    assert_eq!(with(i64::MIN).forced_stop(), None);
    assert_eq!(with(i64::MAX).forced_stop(), Some(i64::MAX));
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
    // Below, at and above one half, on both signs.
    assert_eq!(div_round_half_away(4, 10), 0);
    assert_eq!(div_round_half_away(5, 10), 1);
    assert_eq!(div_round_half_away(6, 10), 1);
    assert_eq!(div_round_half_away(-4, 10), 0);
    assert_eq!(div_round_half_away(-5, 10), -1);
    assert_eq!(div_round_half_away(-6, 10), -1);
    // Exact quotients are untouched; a divisor of one is the identity.
    assert_eq!(div_round_half_away(-9, 3), -3);
    assert_eq!(div_round_half_away(i64::MIN, 1), i64::MIN);
    assert_eq!(div_round_half_away(i64::MAX, 1), i64::MAX);
    // The extremes never overflow.
    assert_eq!(div_round_half_away(i64::MAX, 2), i64::MAX / 2 + 1);
    assert_eq!(div_round_half_away(i64::MIN, 2), i64::MIN / 2);
    assert_eq!(div_round_half_away(i64::MIN + 1, 2), i64::MIN / 2);
    assert_eq!(div_round_half_away(i64::MAX, i64::MAX), 1);
    assert_eq!(div_round_half_away(i64::MIN, i64::MAX), -1);
    // A divisor of zero or below is a caller defect and answers zero.
    assert_eq!(div_round_half_away(7, 0), 0);
    assert_eq!(div_round_half_away(7, -2), 0);
    // DIFFERENTIAL: every small numerator and divisor against the exact
    // rational rounding, `2|r| >= d` away from zero, computed in i128.
    for d in 1_i64..=12 {
        for n in -60_i64..=60 {
            let (q, r) = (i128::from(n) / i128::from(d), i128::from(n) % i128::from(d));
            let want = if 2 * r.abs() >= i128::from(d) {
                q + r.signum()
            } else {
                q
            };
            assert_eq!(i128::from(div_round_half_away(n, d)), want, "{n}/{d}");
        }
    }
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
    assert_eq!(
        best_shown([("long", Some((idle, true))), ("short", None)]),
        None
    );
    let busy = traded(3, 10);
    assert_eq!(
        best_shown([("long", Some((idle, true))), ("short", Some((busy, false)))]),
        Some(("short", busy, false))
    );
    // Neither side shown, or both idle: no row.
    assert_eq!(best_shown::<&str>([("long", None), ("short", None)]), None);
    assert_eq!(
        best_shown([("long", Some((idle, false))), ("short", Some((idle, true)))]),
        None
    );
    // One trade is enough to be a row.
    let one = traded(1, -5);
    assert_eq!(
        best_shown([("long", Some((one, false))), ("short", None)]),
        Some(("long", one, false))
    );
    // An admitted side beats an unadmitted one however much less it made.
    let rich = traded(9, 1_000_000);
    assert_eq!(
        best_shown([("long", Some((rich, false))), ("short", Some((one, true)))]),
        Some(("short", one, true))
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
    // Order is kept, duplicates of an admitted cell are kept, and an empty
    // grid stays empty.
    let one = traded(1, 3);
    let full = grid::Grid {
        cells: vec![one, idle, busy, idle, busy],
        ..grid::Grid::default()
    };
    assert_eq!(prune_cells(full, envelope).cells, vec![one, busy, busy]);
    assert!(
        prune_cells(grid::Grid::default(), envelope)
            .cells
            .is_empty()
    );
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
    // One point at the largest reference that still converts to one ppm, and
    // one paisa above it.
    assert_eq!(elite_ceiling_ppm(1, || Ok(100_000_000)), Ok(1));
    assert!(elite_ceiling_ppm(1, || Ok(100_000_001)).is_err());
    // A reference of zero or below converts to nothing; so does a negative
    // ceiling; the largest ceiling saturates rather than wrapping.
    assert!(elite_ceiling_ppm(5, || Ok(0)).is_err());
    assert!(elite_ceiling_ppm(5, || Ok(-1)).is_err());
    assert!(elite_ceiling_ppm(-1, || Ok(2_500_000)).is_err());
    assert_eq!(
        elite_ceiling_ppm(i64::MAX, || Ok(1)),
        Ok(points_to_ppm_at(i64::MAX, 1))
    );
    assert!(points_to_ppm_at(i64::MAX, 1) > 0);
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
    for (max_points, top) in [(-1, 10), (i64::MIN, 10), (5, 0), (0, 1_001), (-1, 0)] {
        assert!(
            screen(max_points, top).contains(REFUSAL),
            "{max_points} {top}"
        );
    }
    for (max_points, top) in [(0, 10), (1, 10), (5, 1), (i64::MAX, 1_000)] {
        let page = screen(max_points, top);
        assert!(!page.contains(REFUSAL), "{max_points} {top}: {page}");
    }
}

/// G18-cli-a-14: every points-ladder stop rung is a whole point or more and
/// at most the stop ceiling, over a quiet and a wide synthetic span.
#[test]
fn every_points_rung_is_a_point_or_more_and_within_the_ceiling() {
    // The grid knobs are process-wide; hold them unset for this read.
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for (spread, close) in [
        (100_i64, 2_500_000_i64),
        (5_000, 2_500_000),
        (1, 15_000),
        (1, 10_000),
        (2, 20_000),
        (5, 50_000),
        (10, 100_000),
        // UNIFORM: every bar spans 2,000 paisa at a 10,000-rupee close, so a
        // bar is 2,000 ppm, the step is 100 ppm, and a point is 100 ppm: every
        // rung is an EXACT multiple of a point, the case a ceiling division
        // must not round up. (A negative spread marks this shape.)
        (-1_000, 1_000_000),
    ] {
        let bars: Vec<indicators::Candle> = (0..400)
            .map(|minute| {
                let (up, down) = if spread < 0 {
                    (-spread, -spread)
                } else {
                    (spread * (1 + minute % 7), spread * (1 + minute % 5))
                };
                indicators::Candle {
                    high: close + up,
                    low: close - down,
                    ..candle(minute, close)
                }
            })
            .collect();
        let rungs = stop_rungs_in_points(&bars);
        let ceiling = max_stop_points(&bars);
        // DIFFERENTIAL: rung `i` is `ceil(i * step / per_point)` whole points,
        // computed here in i128 from the same three inputs, then cut at the
        // ceiling. An exact multiple must not round up a point.
        let per_point = i128::from(points_to_ppm_at(1, reference_price(&bars)).max(1));
        let step = i128::from(grid_step_ppm(&bars, 1));
        let want: Vec<i64> = (1..=grid_rungs(&bars))
            .map(|i| {
                let ppm = step * i128::try_from(i).expect("rung index");
                i64::try_from((ppm + per_point - 1) / per_point).expect("points")
            })
            .filter(|&pt| pt <= ceiling)
            .collect();
        assert_eq!(rungs, want, "{spread}");
        if spread < 0 {
            assert_eq!(step, per_point, "premise: one step is one point");
            let whole: Vec<i64> = (1..).take(rungs.len()).collect();
            assert_eq!(rungs, whole, "exact multiples stay whole points");
            assert!(rungs.len() >= 2, "{rungs:?}");
        }
        assert!(!rungs.is_empty(), "{spread}: a ladder");
        assert!(
            rungs.iter().all(|&pt| (1..=ceiling).contains(&pt)),
            "{spread}: {rungs:?} within 1..={ceiling}"
        );
    }
    // No bars: the reference and the ceiling fall back, and the rule holds.
    let rungs = stop_rungs_in_points(&[]);
    assert!(
        rungs
            .iter()
            .all(|&pt| (1..=max_stop_points(&[])).contains(&pt))
    );
}
