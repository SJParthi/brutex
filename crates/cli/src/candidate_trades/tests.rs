//! Adversarial tests against the public writer/readers and the real pricing pass.
#![allow(clippy::expect_used, reason = "fixture failures must fail the test")]

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "brutex-candidate-detail-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
fn fixture() -> &'static (Vec<indicators::Candle>, Column) {
    static FIXTURE: std::sync::OnceLock<(Vec<indicators::Candle>, Column)> =
        std::sync::OnceLock::new();
    FIXTURE.get_or_init(|| {
        let bars = runner::synthetic::sessions(7);
        let column = Column::build(&bars, &mut crate::evaluator().expect("evaluator"));
        assert!(column.len() > 100);
        (bars, column)
    })
}
fn rules() -> crate::Rules {
    crate::Rules {
        max_mae_ppm: 0,
        min_rr_bp: 0,
        min_win_rate_bp: 0,
        min_trades: 0,
        min_assurance_bp: 0,
        min_weakest_bp: 0,
        min_ret_over_dd_bp: 0,
        require_protective_exits: false,
        min_fill_headroom_bp: 0,
        min_avg_rr_bp: 0,
        top: 2,
    }
}
fn tier_input() -> Tier {
    Tier {
        index: 99,
        eligible: 9,
        evaluated: 1,
        horizon: 3,
        rungs: 1,
        step_ppm: Some(100),
        forced_ppm: None,
        ratios: true,
        rules: rules(),
        stops_ppm: vec![100],
    }
}
fn attempt(root: &Path) -> crate::sweep_evidence::Attempt {
    crate::sweep_evidence::begin(root, [42; 32], crate::sweep_evidence::Operation::Audit)
        .expect("durable attempt")
}
fn grids(mask: &ConditionMask) -> [(Direction, grid::Grid); 2] {
    let (bars, column) = fixture();
    [Direction::Long, Direction::Short].map(|direction| {
        let grid = grid::evaluate(
            bars,
            column,
            mask,
            Horizon::bars(3).expect("horizon"),
            crate::side_of_direction(direction),
            grid::Levels {
                rungs: 1,
                step_ppm: Some(100),
                forced: None,
                ratios: true,
                stops_ppm: &[100],
            },
        );
        (direction, grid)
    })
}
fn record_both(capture: &Capture<'_>, tier: &Tier, mask: &ConditionMask) {
    for (direction, grid) in grids(mask) {
        capture
            .record(
                tier,
                &Evaluated {
                    rank: 1,
                    mask,
                    direction,
                    selected: crate::shown_cell(&grid, tier.rules),
                    grid: &grid,
                },
            )
            .expect("exact capture");
    }
}

fn busy_read_refuses(path: &Path, read: impl FnOnce() -> Result<(), String> + Send + 'static) {
    let owner = File::open(path).expect("busy file");
    owner.lock().expect("exclusive fixture lock");
    let (send, receive) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || send.send(read()).expect("read result"));
    let answer = receive.recv_timeout(std::time::Duration::from_millis(500));
    // Release before assertions/join so a regression cannot strand the test.
    owner.unlock().expect("release fixture lock");
    worker.join().expect("reader thread");
    let why = answer
        .expect("busy evidence must refuse without waiting for the owner")
        .expect_err("a held exclusive lock must not expose evidence");
    assert!(
        why.contains("busy"),
        "the actual contention must be named: {why}"
    );
}

#[test]
fn busy_catalog_tier_manifest_and_cold_warm_trade_reads_refuse_without_waiting() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    record_both(&capture, &tier, &ConditionMask::default());
    let summary = capture.finish().expect("complete");
    for path in [
        capture.directory.join("catalog.bin"),
        tier_path(&capture.directory, 0),
        candidate_path(&capture.directory, key_at(0, 0)),
        trade_path(&capture.directory, key_at(0, 0)),
    ] {
        let target = root.clone();
        let pinned = summary.clone();
        busy_read_refuses(&path, move || {
            TradeReader::open(&target, &pinned, key_at(0, 0), DEFAULT_MAX_BYTES).map(|_| ())
        });
    }
    let mut warm =
        TradeReader::open(&root, &summary, key_at(0, 0), DEFAULT_MAX_BYTES).expect("warm");
    busy_read_refuses(&trade_path(&capture.directory, key_at(0, 0)), move || {
        warm.page(0, 1).map(|_| ())
    });
    assert!(
        TradeReader::open(&root, &summary, key_at(0, 0), DEFAULT_MAX_BYTES)
            .expect("same exact evidence remains readable after unlock")
            .page(0, 1)
            .is_ok()
    );
}

#[cfg(unix)]
#[test]
fn candidate_paths_refuse_symbolic_aliases_to_otherwise_valid_evidence() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    record_both(&capture, &tier, &ConditionMask::default());
    let summary = capture.finish().expect("complete");
    for path in [
        capture.directory.join("catalog.bin"),
        trade_path(&capture.directory, key_at(0, 0)),
    ] {
        let backing = path.with_extension("backing");
        fs::rename(&path, &backing).expect("preserve exact bytes");
        std::os::unix::fs::symlink(&backing, &path).expect("alias fixture");
        let refusal = TradeReader::open(&root, &summary, key_at(0, 0), DEFAULT_MAX_BYTES)
            .err()
            .expect("an alias is not a regular immutable evidence path");
        assert!(refusal.contains("candidate detail refused"));
        fs::remove_file(&path).expect("remove fixture alias");
        fs::rename(&backing, &path).expect("restore fixture path");
    }
    assert!(TradeReader::open(&root, &summary, key_at(0, 0), DEFAULT_MAX_BYTES).is_ok());
}

#[test]
fn exact_replay_roundtrips_both_sides_policy_ladders_and_bounded_pages() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    let mask = ConditionMask::default();
    record_both(&capture, &tier, &mask);
    let summary = capture.finish().expect("complete");
    capture.confirm().expect("parent boundary");
    assert_eq!((summary.tiers, summary.candidates), (1, 2));
    assert_eq!(
        super::tier(&root, &summary, 0, DEFAULT_MAX_BYTES).expect("tier read"),
        tier
    );
    let candidates =
        candidates_page(&root, &summary, 0, 0, 2, DEFAULT_MAX_BYTES).expect("metadata page");
    assert_eq!(candidates.len(), 2);
    for (candidate, (direction, grid)) in candidates.iter().zip(grids(&mask)) {
        let (cell, admitted) = crate::shown_cell(&grid, rules()).expect("cell");
        assert_eq!(candidate.cell, Some(cell));
        assert_eq!(candidate.admitted, admitted);
        assert_eq!(candidate.key.direction, direction);
        assert_eq!(candidate.stops, grid.stops.rungs());
        assert_eq!(candidate.targets, grid.targets.rungs());
        assert_eq!(candidate.trails, grid.trails.rungs());
        let expected = grid::materialize_cell(
            bars,
            column,
            &mask,
            Horizon::bars(3).expect("horizon"),
            crate::side_of_direction(direction),
            &grid,
            &cell,
        )
        .expect("reference replay");
        assert!(!expected.is_empty());
        let mut reader = TradeReader::open(&root, &summary, candidate.key, DEFAULT_MAX_BYTES)
            .expect("exact reader");
        let mut actual = Vec::new();
        let mut start = 0;
        while start < cell.trades {
            let page = reader.page(start, 11).expect("page");
            start += page.len() as u64;
            actual.extend(page);
        }
        assert_eq!(actual.len(), expected.len());
        for (stored, replayed) in actual.iter().zip(expected) {
            assert_eq!(
                (
                    stored.signal_bar,
                    stored.entry_bar,
                    stored.exit_bar,
                    stored.worst,
                    stored.best
                ),
                (
                    replayed.signal_bar as u64,
                    replayed.entry_bar as u64,
                    replayed.exit_bar as u64,
                    replayed.worst,
                    replayed.best
                )
            );
        }
        assert!(reader.page(cell.trades + 1, 1).is_err());
        assert!(reader.page(0, 257).is_err());
        assert!(TradeReader::open(&root, &summary, candidate.key, 48).is_err());
    }
    assert!(candidates_page(&root, &summary, 0, 0, 0, DEFAULT_MAX_BYTES).is_err());
    assert!(candidates_page(&root, &summary, 0, 3, 1, DEFAULT_MAX_BYTES).is_err());
    assert!(read(&root, [42; 32], attempt.token(), 48).is_err());
}

#[test]
fn repeated_exact_publication_is_idempotent_and_conflicting_selection_poisoned() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let mask = ConditionMask::default();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    record_both(&capture, &tier, &mask);
    record_both(&capture, &tier, &mask);
    let prior = fs::read(candidate_path(&capture.directory, key_at(0, 0))).expect("bytes");
    record_both(&capture, &tier, &mask);
    assert_eq!(
        fs::read(candidate_path(&capture.directory, key_at(0, 0))).expect("bytes"),
        prior
    );
    let grid = grids(&mask).into_iter().next().expect("long").1;
    assert!(
        capture
            .record(
                &tier,
                &Evaluated {
                    rank: 1,
                    mask: &mask,
                    direction: Direction::Long,
                    selected: None,
                    grid: &grid
                }
            )
            .is_err()
    );
    assert!(capture.finish().is_err());
    assert!(
        read(&root, [42; 32], attempt.token(), DEFAULT_MAX_BYTES)
            .expect("read")
            .is_none()
    );
}

#[test]
fn no_cell_outcomes_and_multiple_tiers_remain_explicit() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let empty = grid::Grid::default();
    let mask = ConditionMask::default().with_bit(383);
    for forced in [None, Some(500)] {
        let mut input = tier_input();
        input.forced_ppm = forced;
        let tier = capture.tier(input).expect("tier");
        for direction in [Direction::Long, Direction::Short] {
            capture
                .record(
                    &tier,
                    &Evaluated {
                        rank: 1,
                        mask: &mask,
                        direction,
                        selected: None,
                        grid: &empty,
                    },
                )
                .expect("no-cell capture");
        }
    }
    let summary = capture.finish().expect("completed");
    assert_eq!(summary.candidates, 4);
    for index in 0..2 {
        let candidates =
            candidates_page(&root, &summary, index, 0, 2, DEFAULT_MAX_BYTES).expect("page");
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.cell.is_none() && !candidate.admitted)
        );
        let mut reader = TradeReader::open(&root, &summary, key_at(index, 0), DEFAULT_MAX_BYTES)
            .expect("empty trade authority");
        assert!(reader.page(0, 1).expect("explicit zero").is_empty());
    }
}

#[test]
fn missing_corrupt_and_replaced_acknowledged_children_cannot_seal_or_confirm() {
    for after in [false, true] {
        for fault in 0..4 {
            let root = root();
            let attempt = attempt(&root);
            let (bars, column) = fixture();
            let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
            let tier = capture.tier(tier_input()).expect("tier");
            record_both(&capture, &tier, &ConditionMask::default());
            if after {
                capture.finish().expect("initial seal");
            }
            let key = key_at(0, 0);
            let path = if fault == 0 {
                tier_path(&capture.directory, 0)
            } else if fault == 1 {
                candidate_path(&capture.directory, key)
            } else {
                trade_path(&capture.directory, key)
            };
            match fault {
                0 | 1 => fs::remove_file(&path).expect("delete acknowledged evidence"),
                2 => OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .expect("file")
                    .set_len(HEADER as u64)
                    .expect("torn payload"),
                _ => {
                    fs::copy(trade_path(&capture.directory, key_at(0, 1)), &path)
                        .expect("foreign side replacement");
                }
            }
            assert!(
                if after {
                    capture.confirm()
                } else {
                    capture.finish().map(|_| ())
                }
                .is_err(),
                "after={after}, fault={fault}"
            );
            if !after {
                assert!(!capture.directory.join("catalog.bin").exists());
            }
        }
    }
}

#[test]
fn warm_reader_refuses_same_length_mutation_and_atomic_replacement() {
    for replace in [false, true] {
        let root = root();
        let attempt = attempt(&root);
        let (bars, column) = fixture();
        let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
        let tier = capture.tier(tier_input()).expect("tier");
        record_both(&capture, &tier, &ConditionMask::default());
        let summary = capture.finish().expect("seal");
        let mut reader =
            TradeReader::open(&root, &summary, key_at(0, 0), DEFAULT_MAX_BYTES).expect("reader");
        assert!(!reader.page(0, 1).expect("initial page").is_empty());
        let path = trade_path(&capture.directory, key_at(0, 0));
        if replace {
            let replacement = capture.directory.join("replacement.bin");
            fs::copy(&path, &replacement).expect("copy");
            fs::rename(replacement, &path).expect("atomic replacement");
        } else {
            let mut file = OpenOptions::new().write(true).open(&path).expect("file");
            file.seek(SeekFrom::Start(HEADER as u64 + 72))
                .expect("seek");
            file.write_all(&i64::MAX.to_le_bytes())
                .expect("same extent corruption");
            file.sync_all().expect("sync");
        }
        assert!(reader.page(0, 1).is_err());
    }
}

#[test]
fn concurrent_same_capture_publications_and_same_identity_attempts_keep_exact_children() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| record_both(&capture, &tier, &ConditionMask::default()));
        }
    });
    let summary = capture.finish().expect("sealed exact duplicate writes");
    assert_eq!(summary.candidates, 2);
    let starts: Vec<_> = (0..8).map(|_| attempt_for_thread(&root)).collect();
    std::thread::scope(|scope| {
        for attempt in &starts {
            let root = &root;
            scope.spawn(move || {
                let capture =
                    Capture::begin(root, attempt, bars, column).expect("independent attempt");
                let tier = capture.tier(tier_input()).expect("tier");
                record_both(&capture, &tier, &ConditionMask::default());
                let summary = capture.finish().expect("seal");
                assert_eq!(
                    read(root, [42; 32], attempt.token(), DEFAULT_MAX_BYTES)
                        .expect("read")
                        .expect("catalog")
                        .digest,
                    summary.digest
                );
            });
        }
    });
}
fn attempt_for_thread(root: &Path) -> crate::sweep_evidence::Attempt {
    attempt(root)
}

fn scored_masks(column: &Column) -> Vec<runner::rank::Scored> {
    let mut found = Vec::new();
    for bit in 0..ConditionMask::BITS {
        let mask = ConditionMask::default().with_bit(bit);
        let hits = column.bits().iter().filter(|bar| bar.hits(&mask)).count();
        if hits > 20
            && hits < column.len() / 2
            && found
                .iter()
                .all(|old: &runner::rank::Scored| old.hits != hits as u64)
        {
            found.push(runner::rank::Scored {
                mask,
                hits: hits as u64,
                edge: runner::outcome::Edge::default(),
            });
            if found.len() == 2 {
                break;
            }
        }
    }
    assert_eq!(
        found.len(),
        2,
        "two independently firing fixture candidates"
    );
    found
}

#[test]
fn real_screen_keeps_nonwinning_candidate_traces_and_actual_cap_across_tiers() {
    let _serial = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_GRID_RUNGS", "1");
    crate::knobs::set("BRUTEX_SCREEN_CAP", "2");
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let scored = scored_masks(column);
    let refs: Vec<_> = scored.iter().collect();
    let first = crate::screen(
        bars,
        column,
        &refs,
        Horizon::bars(3).expect("horizon"),
        rules(),
        crate::Pricing {
            recording: None,
            capture: Some(&capture),
        },
    )
    .expect("real first pricing pass");
    let selected = first.selected.expect("one candidate chosen");
    let mut stricter = rules();
    stricter.min_win_rate_bp = 10_001;
    let second = crate::screen(
        bars,
        column,
        &refs,
        Horizon::bars(3).expect("horizon"),
        stricter,
        crate::Pricing {
            recording: None,
            capture: Some(&capture),
        },
    )
    .expect("real second pricing pass");
    assert!(!second.admitted_any);
    let summary = capture.finish().expect("sealed");
    assert_eq!((summary.tiers, summary.candidates), (2, 8));
    let candidates = candidates_page(&root, &summary, 0, 0, 4, DEFAULT_MAX_BYTES)
        .expect("all priced candidates");
    let winner = candidates
        .iter()
        .find(|c| {
            c.mask_words == selected.scored.mask.words() && c.key.direction == selected.direction
        })
        .expect("exact winner");
    let other = candidates
        .iter()
        .find(|c| {
            c.mask_words != selected.scored.mask.words()
                && c.cell.is_some_and(|cell| cell.trades > 0)
        })
        .expect("priced nonwinner");
    let mut winner_rows =
        TradeReader::open(&root, &summary, winner.key, DEFAULT_MAX_BYTES).expect("winner trades");
    let mut other_rows =
        TradeReader::open(&root, &summary, other.key, DEFAULT_MAX_BYTES).expect("nonwinner trades");
    let trace = |rows: Vec<crate::trades::Row>| {
        rows.into_iter()
            .map(|row| (row.signal_bar, row.entry_bar, row.exit_bar, row.worst))
            .collect::<Vec<_>>()
    };
    assert_ne!(
        trace(winner_rows.page(0, 256).expect("winner page")),
        trace(other_rows.page(0, 256).expect("nonwinner page"))
    );
    assert_eq!(
        super::tier(&root, &summary, 1, DEFAULT_MAX_BYTES)
            .expect("tier")
            .rules,
        stricter
    );
    crate::knobs::clear_all();
}

#[test]
fn actual_screen_callback_failure_refuses_the_pass_and_never_seals_catalog() {
    let _serial = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_GRID_RUNGS", "1");
    crate::knobs::set("BRUTEX_SCREEN_CAP", "2");
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    fs::write(
        trade_path(&capture.directory, key_at(0, 0)),
        b"deliberate immutable-file fault",
    )
    .expect("fault");
    let scored = scored_masks(column);
    let refs: Vec<_> = scored.iter().collect();
    let result = crate::screen(
        bars,
        column,
        &refs,
        Horizon::bars(3).expect("horizon"),
        rules(),
        crate::Pricing {
            recording: None,
            capture: Some(&capture),
        },
    );
    assert!(result.is_err());
    assert!(capture.finish().is_err());
    assert!(!capture.directory.join("catalog.bin").exists());
    assert!(!crate::results::Results::path(&root).exists());
    crate::knobs::clear_all();
}

#[test]
fn actual_audit_refuses_a_failed_capture_before_publishing_its_parent() {
    let _serial = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_GRID_RUNGS", "1");
    crate::knobs::set("BRUTEX_SCREEN_CAP", "2");
    crate::knobs::set("BRUTEX_KEEP", "2");
    let root = root();
    fs::create_dir_all(root.join("results")).expect("root");
    fs::write(
        root.join("results/candidate-trades-v1"),
        b"capture directory deliberately obstructed",
    )
    .expect("capture fault");
    let (bars, column) = fixture();
    let key = crate::stored::swept_index("NIFTY").expect("fixture index");
    let id = runner::identity::identity(&runner::identity::Run {
        mask: ConditionMask::default(),
        direction: runner::identity::Direction::Undirected,
        instrument: &key,
        timeframe: "1min",
        params: runner::identity::Params::of(engine::Ladder::with_min_hits(500)),
        data_digest: runner::identity::data_digest(bars),
        commit: "candidate-boundary-test",
        feed: "zerodha",
    });
    let report = crate::audit_bars(
        &crate::evaluator(),
        bars.clone(),
        "GENERATED TEST FIXTURE",
        500,
        Some(&id),
        crate::AuditOptions {
            prepared_column: Some(column.clone()),
            replay: None,
            execution: None,
            native_minute_execution: true,
            recording: Some(crate::Recording {
                root: &root,
                feed: "zerodha",
                underlying: "NIFTY",
                timeframe: "1min",
                from: (2024, 1),
                to: (2024, 1),
                attempt: None,
                months_asked: 1,
                months_found: 1,
            }),
            rules: rules(),
            lens: runner::rank::Lens::Detectability,
            ceiling: Some(50_000),
            validate: false,
        },
    );
    crate::knobs::clear_all();
    assert!(
        report.contains("candidate capture did not start"),
        "must reach the actual capture gate, not an unrelated refusal: {report}"
    );
    assert!(
        !crate::results::Results::path(&root).exists(),
        "child failure must prevent the real publication branch"
    );
    let evidence = crate::sweep_evidence::read(&root, id.bytes(), DEFAULT_MAX_BYTES)
        .expect("lifecycle")
        .expect("attempt");
    assert_eq!(
        evidence.completion,
        crate::sweep_evidence::Completion::Refused
    );
}

#[test]
fn incomplete_catalog_reservations_limits_and_corrupt_footer_refuse_loudly() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    assert!(
        capture.finish().is_err(),
        "expected sides cannot disappear into an empty catalog"
    );
    assert!(
        capture
            .record(
                &tier,
                &Evaluated {
                    rank: 1,
                    mask: &ConditionMask::default(),
                    direction: Direction::Long,
                    grid: &grid::Grid::default(),
                    selected: None
                }
            )
            .is_err(),
        "a failed capture remains poisoned"
    );
    let root = root_for_limit();
    let attempt = attempt_for_thread(&root);
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let mut input = tier_input();
    input.eligible = u64::MAX;
    input.evaluated = u64::MAX;
    assert!(capture.tier(input).is_err());
    let root = root_for_limit();
    let attempt = attempt_for_thread(&root);
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    record_both(&capture, &tier, &ConditionMask::default());
    let summary = capture.finish().expect("seal");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(capture.directory.join("catalog.bin"))
        .expect("catalog");
    file.seek(SeekFrom::End(-1)).expect("seal seek");
    let mut byte = [0];
    file.read_exact(&mut byte).expect("read footer byte");
    file.seek(SeekFrom::End(-1)).expect("seal seek");
    let [prior] = byte;
    file.write_all(&[prior ^ 1]).expect("corrupt footer");
    file.sync_all().expect("sync");
    assert!(read(&root, [42; 32], attempt.token(), DEFAULT_MAX_BYTES).is_err());
    assert!(candidates_page(&root, &summary, 0, 0, 1, DEFAULT_MAX_BYTES).is_err());
}

#[test]
fn final_confirmation_never_recreates_a_missing_sealed_catalog() {
    let root = root();
    let attempt = attempt(&root);
    let (bars, column) = fixture();
    let capture = Capture::begin(&root, &attempt, bars, column).expect("capture");
    let tier = capture.tier(tier_input()).expect("tier");
    record_both(&capture, &tier, &ConditionMask::default());
    capture.finish().expect("initial seal");
    let path = capture.directory.join("catalog.bin");
    std::fs::remove_file(&path).expect("simulate lost seal");
    assert!(capture.confirm().is_err());
    assert!(!path.exists(), "confirmation cannot repair lost evidence");
    assert!(capture.finish().is_err(), "failure remains latched");
}

#[test]
fn expression_capture_replays_or_not_without_relabelling_the_same_referenced_and_bits() {
    let (bars, column) = fixture();
    let facts = runner::trade::SliceFacts::of(bars, column);
    for (source, has_trades) in [("0 | !0", true), ("0 & !0", false)] {
        let expression = Expression::parse(source).expect("predicate");
        let root = root();
        let attempt = attempt(&root);
        let capture = Capture::begin_expression(&root, &attempt, bars, column, &expression)
            .expect("explicit expression capture");
        let tier = capture.tier(tier_input()).expect("tier");
        for direction in [Direction::Long, Direction::Short] {
            let priced = grid::evaluate_expression_over(
                bars,
                column,
                &expression,
                Horizon::bars(3).expect("horizon"),
                crate::side_of_direction(direction),
                grid::Levels {
                    rungs: 1,
                    step_ppm: Some(100),
                    forced: None,
                    ratios: true,
                    stops_ppm: &[100],
                },
                &facts,
            )
            .expect("expression pricing");
            let selected = crate::shown_cell(&priced, tier.rules);
            assert_eq!(selected.is_some(), has_trades, "{source}, {direction:?}");
            capture
                .record(
                    &tier,
                    &Evaluated {
                        rank: 1,
                        mask: &expression.referenced(),
                        direction,
                        grid: &priced,
                        selected,
                    },
                )
                .expect("exact expression rows");
        }
        let summary = capture.finish().expect("sealed expression");
        capture.confirm().expect("publication proof");
        assert_eq!(summary.model, Model::Expression);
        assert_eq!(
            summary.expression().expect("complete program").encode(),
            expression.encode()
        );
        assert!(
            read(&root, [42; 32], attempt.token(), DEFAULT_MAX_BYTES)
                .expect("AND namespace")
                .is_none()
        );
        let saved = read_model(
            &root,
            [42; 32],
            attempt.token(),
            Model::Expression,
            DEFAULT_MAX_BYTES,
        )
        .expect("expression namespace")
        .expect("saved");
        for candidate in candidates_page(&root, &saved, 0, 0, 2, DEFAULT_MAX_BYTES).expect("page") {
            assert_eq!(candidate.mask_words, expression.referenced().words());
            assert_ne!(
                candidate.trade_identity,
                candidate_identity(&candidate, None),
                "domain includes the full program"
            );
            let mut reader = TradeReader::open(&root, &saved, candidate.key, DEFAULT_MAX_BYTES)
                .expect("exact trace");
            assert_eq!(!reader.page(0, 256).expect("rows").is_empty(), has_trades);
        }
        let changed = Expression::parse(if has_trades { "0 & !0" } else { "0 | !0" })
            .expect("different program same referenced bits");
        assert!(
            Capture::begin_expression(&root, &attempt, bars, column, &changed).is_err(),
            "an existing attempt cannot change its predicate"
        );
    }
}
fn root_for_limit() -> PathBuf {
    root()
}
