//! Every fixed-width table renderer in this crate, driven at the extremes its
//! figures can reach: `i64::MIN` and `i64::MAX` paisa, `u64::MAX` counts, the
//! longest label. Each one must keep its columns apart and under their
//! headers. D-1420.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a failed fixture must fail its test"
)]

use crate::columns::{Align, assert_under};

const L: Align = Align::Left;
const R: Align = Align::Right;

/// The line that starts with `head` (after indentation) and the `count` lines
/// that follow it.
fn table<'a>(text: &'a str, head: &str, count: usize) -> (&'a str, Vec<&'a str>) {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.trim_start().starts_with(head))
        .unwrap_or_else(|| panic!("no `{head}` header in\n{text}"));
    (lines[at], lines[at + 1..=at + count].to_vec())
}

fn under(header: &str, rows: &[&str], aligns: &[Align]) {
    for row in rows {
        if let Err(why) = assert_under(header, row, aligns) {
            panic!("{why}");
        }
    }
}

fn frontier_row(rank: u16, extreme: i64, count: u64) -> crate::frontier::Row {
    crate::frontier::Row {
        direction: costs::fill::Direction::Long,
        rules: crate::Rules {
            max_mae_ppm: i64::MAX,
            min_rr_bp: 0,
            min_win_rate_bp: 0,
            min_assurance_bp: 0,
            min_weakest_bp: 0,
            min_trades: 0,
            min_ret_over_dd_bp: 0,
            min_fill_headroom_bp: 0,
            min_avg_rr_bp: 0,
            require_protective_exits: false,
            top: 25,
        },
        identity: [7; 32],
        rank,
        mask_words: [1, 0, 0, 0, 0, 0],
        hits: count,
        n: count,
        mean_milli_paisa: extreme,
        t_milli: extreme,
        payoff_bp: extreme,
        wins: count,
        trades: count,
        cell_wins: count,
        pessimistic: extreme,
        worst_trade: extreme,
        max_drawdown: extreme,
        min_win: extreme,
        gross_win: extreme,
        gross_loss: extreme,
    }
}

#[test]
fn the_top_combinations_table_keeps_extreme_figures_apart_and_under_their_headers() {
    let mut record = crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES]);
    record.underlying = crate::results::field("NSE-NIFTY");
    record.feed = crate::results::field("zerodha");
    record.timeframe = crate::results::field("5min");
    let found = [
        frontier_row(u16::MAX, i64::MIN, u64::MAX),
        frontier_row(1, i64::MAX - 1, 0),
        frontier_row(2, 0, 1),
    ];
    let text = crate::render_top_record(&record, &found, None, "");
    let (header, rows) = table(&text, "rank", found.len());
    under(header, &rows, &[L, R, R, R, R, R]);
}

/// A ledger record at the extremes: every money figure at `extreme`, every
/// count at `count`, and the two 16-byte text fields full.
fn extreme_record(extreme: i64, count: u64) -> crate::results::Record {
    crate::results::Record {
        identity: [9; 32],
        finished_micros: 1,
        feed: crate::results::field("abcdefghijklmnop"),
        underlying: crate::results::field("NSE-NIFTY"),
        timeframe: crate::results::field("qrstuvwxyzabcdef"),
        from_year: 2026,
        from_month: 1,
        to_year: 2026,
        to_month: 1,
        months_asked: u32::MAX,
        months_found: u32::MAX,
        bars: count,
        min_hits: count,
        combinations: count,
        depth: u32::MAX,
        halted: 0,
        trades: count,
        pessimistic: extreme,
        optimistic: extreme,
        worst_trade: extreme,
        max_drawdown: extreme,
        winner_mae: extreme,
        winner_mfe: extreme,
        all_mae: extreme,
        exit_rungs: [-1; 5],
        mask_words: [0; 6],
    }
}

#[test]
fn the_results_listing_keeps_full_text_fields_and_extreme_figures_apart() {
    let root = std::fs::canonicalize(crate::verification_scratch().expect("scratch"))
        .expect("canonical scratch");
    {
        let mut ledger = crate::results::Results::open(&root).expect("ledger");
        for (id, record) in [
            extreme_record(i64::MIN, u64::MAX),
            extreme_record(i64::MAX, 0),
            extreme_record(0, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let record = crate::results::Record {
                identity: [u8::try_from(id).expect("three rows"); 32],
                ..record
            };
            ledger.append(&record).expect("append");
        }
    }
    let text = crate::results_at(&root, None, None);
    let _ = std::fs::remove_dir_all(&root);
    let (header, rows) = table(&text, "feed", 3);
    under(header, &rows, &[L, L, R, R, R, R, R, R, R, R]);
}

#[test]
fn the_screen_table_keeps_extreme_cells_apart_and_under_their_headers() {
    use runner::outcome::Edge;
    use runner::rank::Scored;
    let mut scored = Scored {
        mask: vocab::ConditionMask::default(),
        hits: 1,
        edge: Edge::default(),
    };
    scored.mask = scored.mask.with_bit(1);
    let cell = |extreme: i64, count: u64| runner::grid::Cell {
        stop: Some(624),
        target: Some(624),
        tsl: Some(624),
        trades: count,
        wins: count,
        pessimistic: extreme,
        optimistic: extreme,
        worst_mae: extreme,
        gross_win: i64::MAX / 100,
        gross_loss: -1,
        ..runner::grid::Cell::default()
    };
    let row = |extreme: i64, count: u64, rank: usize| crate::Screened {
        side: costs::fill::Direction::Short,
        scored: &scored,
        rank,
        cell: cell(extreme, count),
        tightest: Some(cell(extreme, count)),
        admitted: true,
        consistency: None,
        steady: true,
        calendar_unmeasured: false,
    };
    let rows = [
        row(i64::MIN, u64::MAX, usize::MAX),
        row(i64::MAX, 1, 99_999),
        row(-1, 1, 1),
    ];
    let mut rules = crate::Rules::operator();
    rules.top = rows.len();
    let mut text = String::new();
    crate::screen_table(&mut text, &rows, rules, 2_500_000);
    let (header, body) = table(&text, "rank", rows.len());
    under(header, &body, &[L, R, R, R, R, R, R, R, R, R]);
}

#[test]
fn the_consistency_table_keeps_extreme_shares_and_the_worst_day_apart() {
    use runner::outcome::Edge;
    use runner::rank::Scored;
    let scored = Scored {
        mask: vocab::ConditionMask::default(),
        hits: 1,
        edge: Edge::default(),
    };
    let row = |share: i64, worst: i128, years: usize, rank: usize| crate::Screened {
        side: costs::fill::Direction::Long,
        scored: &scored,
        rank,
        cell: runner::grid::Cell::default(),
        tightest: None,
        admitted: true,
        consistency: Some(crate::Consistency {
            shares_bp: [share; crate::stability::GRAINS.len()],
            worst_day: worst,
            years,
        }),
        steady: true,
        calendar_unmeasured: false,
    };
    let rows = [
        row(i64::MIN, i128::from(i64::MIN), usize::MAX, usize::MAX),
        row(i64::MAX, i128::from(i64::MAX), 1, 2),
        row(10_000, -1, 1, 3),
    ];
    let mut text = String::new();
    crate::append_consistency(&mut text, &rows, rows.len());
    let (header, body) = table(&text, "rank", rows.len());
    // `worst day` is one header of the same width as `worst_day`.
    under(
        &header.replace("worst day", "worst_day"),
        &body,
        &[L, R, R, R, R, R, R, R, R, R, R],
    );
}

#[test]
fn the_descent_table_keeps_extreme_figures_apart_and_under_their_headers() {
    let mut text = String::new();
    crate::descent_table(
        &mut text,
        vec![
            (u64::MAX, Ok(extreme_record(i64::MIN, u64::MAX))),
            (1_000, Ok(extreme_record(i64::MAX, 0))),
            (10, Ok(extreme_record(0, 1))),
        ],
    );
    let (header, rows) = table(&text, "support", 3);
    under(header, &rows, &[L, R, R, R, R, R, R, R, R]);
}

/// v4-3, D-1487: the `descend` progress line on stderr has no header to sit
/// under (it is printed as each step lands), so what it must keep is apart:
/// every figure its own word, at every extreme, and a refusal never touching
/// its support.
#[test]
fn the_descent_progress_line_keeps_extreme_figures_apart() {
    for (support, record) in [
        (u64::MAX, extreme_record(i64::MIN, u64::MAX)),
        (1_000, extreme_record(i64::MAX, 0)),
        (10, extreme_record(0, 1)),
    ] {
        let line = crate::descent_line(support, Ok(record));
        let words: Vec<&str> = line.split_whitespace().collect();
        let wanted = [
            format!("{support}ppm"),
            record.min_hits.to_string(),
            record.combinations.to_string(),
            record.depth.to_string(),
            if record.halted == 0 { "yes" } else { "NO" }.to_owned(),
            record.trades.to_string(),
            record.winner_mae.to_string(),
            record.all_mae.to_string(),
            record.pessimistic.to_string(),
        ];
        assert_eq!(words, wanted, "every figure is its own word: {line:?}");
    }
    let refused = crate::descent_line(u64::MAX, Err("no bars\nsecond line".to_owned()));
    assert_eq!(
        refused,
        format!("  {}ppm REFUSED: no bars", u64::MAX),
        "a 23-character support does not touch its refusal"
    );
}

#[test]
fn the_range_table_keeps_extreme_figures_apart_and_under_their_headers() {
    let row = |rung: &'static str, record| crate::RungRow {
        rung,
        outcome: Ok(record),
        missing: Vec::new(),
        excluded: crate::stored::CalendarExclusion::none(),
        retention: None,
        validation: None,
    };
    let rows = [
        row("15min", extreme_record(i64::MIN, u64::MAX)),
        row("1day", extreme_record(i64::MAX, 0)),
        row("1min", extreme_record(-1, 1)),
    ];
    let mut text = String::new();
    crate::range_table(&mut text, &rows);
    let (header, body) = table(&text, "rung", rows.len());
    under(header, &body, &[L, R, R, R, R, R, R, R, R, R, R]);
}

#[test]
fn the_window_shape_table_keeps_an_extreme_train_width_apart() {
    let validated = |train_bars: usize| runner::validate::Validated {
        folds: vec![runner::validate::FoldResult {
            train_bars,
            ..runner::validate::FoldResult::default()
        }],
        refused: None,
    };
    let text = crate::decay_block(&validated(usize::MAX), &validated(1));
    let (header, rows) = table(&text, "shape", 2);
    // `train bars` is one header of the same width as `train_bars`.
    under(
        &header.replace("train bars", "train_bars"),
        &rows,
        &[L, R, R, R, R],
    );
}
