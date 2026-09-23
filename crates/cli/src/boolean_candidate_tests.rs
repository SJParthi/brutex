#![cfg(test)]
//! Generated complete-calendar stored fixtures, never market profitability proof.
use super::*;
use runner::exit_grid_policy::{
    ExecutionResolutionV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1,
    RationalPercentileV1, RungPlanV1, printed_ohlcv_cost_model_id_v1,
};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn config(root: &Path) -> Result<StrictConfig, String> {
    StrictConfig::from_values(
        Some(root.as_os_str().to_owned()),
        Some("4194304".into()),
        Some("40000".into()),
    )
    .map_err(display)
}
pub(crate) struct Fixture {
    pub(crate) root: PathBuf,
    pub(crate) output: PathBuf,
}
impl Fixture {
    pub(crate) fn new() -> Result<Self, String> {
        let root = std::env::temp_dir().join(format!(
            "brutex-boolean-candidate-generated-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).map_err(display)?;
        let output = root.join("results");
        fs::create_dir(&output).map_err(display)?;
        Ok(Self { root, output })
    }
    pub(crate) fn prepare(&self, name: &str) -> Result<(), String> {
        let key = crate::stored::swept_index(name)?;
        let symbol = brutex_core::universe::fnv1a(name).to_le_bytes();
        let symbol = u32::from_le_bytes(
            symbol
                .get(..4)
                .ok_or("fixture symbol prefix")?
                .try_into()
                .map_err(|_| "fixture symbol width")?,
        );
        for month in [4, 5, 6, 7, 8, 9] {
            let mut minutes = Vec::new();
            let mut daily = Vec::new();
            for day in 1..=31 {
                let Ok(date) = pull::session::Day::new(2025, month, day) else {
                    continue;
                };
                let stamp = i64::from(date.days_from_epoch());
                let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(stamp) else {
                    continue;
                };
                if indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&stamp) {
                    continue;
                }
                let start = minutes.len();
                for window in session.windows.iter().take(usize::from(session.count)) {
                    for minute in window.from..=window.to {
                        let open = 2_000_000 + i64::from((minute + u16::from(day)) % 31) * 100;
                        minutes.push(Bar {
                            ts_micros: stamp * 86_400_000_000 + i64::from(minute) * 60_000_000
                                - indicators::IST_OFFSET_MICROS,
                            open,
                            high: open + 1000,
                            low: open - 900,
                            close: open + if minute % 3 == 0 { -50 } else { 50 },
                            volume: 100,
                            open_interest: i64::MIN,
                        });
                    }
                }
                daily.push(aggregate(
                    minutes.get(start..).ok_or("fixture session slice")?,
                )?);
            }
            for (rung, bars) in [
                (Timeframe::MINUTE_1, minutes.as_slice()),
                (Timeframe::DAY_1, daily.as_slice()),
            ] {
                let path = StorePath::for_key(
                    brutex_core::vendor::Vendor::Zerodha,
                    &key,
                    rung,
                    YearMonth::new(2025, month).map_err(display)?,
                    FileKind::Bars,
                )
                .map_err(display)?;
                if path.to_path_buf(&self.root).exists() {
                    continue;
                }
                let mut writer =
                    BarFile::open_or_create(&self.root, path, symbol).map_err(display)?;
                writer.append(bars).map_err(display)?;
            }
        }
        Ok(())
    }
    pub(crate) fn produce(
        &self,
        name: &str,
        programs: &[Expression],
    ) -> Result<CommittedBooleanFamilyV1, String> {
        self.produce_month(name, programs, 5)
    }
    pub(crate) fn produce_month(
        &self,
        name: &str,
        programs: &[Expression],
        month: u8,
    ) -> Result<CommittedBooleanFamilyV1, String> {
        self.produce_span(name, programs, month, month)
    }
    pub(crate) fn produce_span(
        &self,
        name: &str,
        programs: &[Expression],
        from: u8,
        to: u8,
    ) -> Result<CommittedBooleanFamilyV1, String> {
        self.prepare(name)?;
        let config = config(&self.root)?;
        let long = policy(Side::Long)?;
        let short = policy(Side::Short)?;
        let mut request = self.request(name, programs, &config, &long, &short)?;
        request.from = (2025, from);
        request.to = (2025, to);
        if from != to {
            // Two complete months retain every priced coordinate and trade;
            // their larger explicit fixture budget changes no production cap.
            request.bounds.trades = 1_000_000;
            request.bounds.bytes = 128 * 1024 * 1024;
        }
        produce_identified(request, "generated-boolean-candidate-fixture")
    }
    fn request<'a>(
        &'a self,
        name: &'a str,
        programs: &'a [Expression],
        config: &'a StrictConfig,
        long: &'a ExitGridPolicyV1,
        short: &'a ExitGridPolicyV1,
    ) -> Result<Request<'a>, String> {
        Ok(Request {
            store: &self.root,
            output: &self.output,
            vendor: brutex_core::vendor::Vendor::Zerodha,
            underlying: name,
            rung: "1min",
            from: (2025, 5),
            to: (2025, 5),
            horizon: Horizon::bars(5).ok_or("horizon")?,
            programs,
            long,
            short,
            inputs: config,
            bounds: Bounds {
                programs: 8,
                coordinates: 512,
                trades: 200_000,
                bytes: 33_554_432,
            },
            widths: indicators::evaluator::Widths::pinned().map_err(display)?,
            thresholds: indicators::pattern::Thresholds::CLASSICAL,
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn aggregate(rows: &[Bar]) -> Result<Bar, String> {
    let first = rows.first().ok_or("empty generated session")?;
    Ok(Bar {
        ts_micros: first.ts_micros,
        open: first.open,
        high: rows.iter().map(|bar| bar.high).max().ok_or("high")?,
        low: rows.iter().map(|bar| bar.low).min().ok_or("low")?,
        close: rows.last().ok_or("close")?.close,
        volume: rows.iter().map(|bar| bar.volume).sum(),
        open_interest: i64::MIN,
    })
}
pub(crate) fn policy(side: Side) -> Result<ExitGridPolicyV1, String> {
    let rank = RationalPercentileV1::new(1, 2).map_err(display)?;
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        side,
        RungPlanV1::new(vec![rank], vec![rank], vec![rank], 1).map_err(display)?,
        RatioLimitsV1::new(1, 1_000_000, 1).map_err(display)?,
        32,
        ExitGridSelectorV1::PessimisticTotal,
        printed_ohlcv_cost_model_id_v1(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .map_err(display)
}
pub(crate) fn programs() -> Result<Vec<Expression>, String> {
    ["30 & 44", "30 | 44", "146 | !146"]
        .iter()
        .map(|raw| Expression::parse(raw).map_err(|why| format!("{why:?}")))
        .collect()
}

#[test]
fn strict_boolean_catalog_keeps_every_coordinate_exact_rows_and_unknowns_and_reuses()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let programs = programs()?;
    let first = fixture.produce("NIFTY", &programs)?;
    assert_eq!(first.programs(), programs);
    assert!(!first.sessions().is_empty());
    assert!(!first.rows().is_empty());
    let mut identities = std::collections::HashSet::new();
    let mut groups = [[0_u64; 2]; 3];
    for row in first.rows() {
        assert!(identities.insert(row.identity()));
        let side = usize::from(row.side() == Side::Short);
        let count = groups
            .get_mut(row.program_index())
            .and_then(|group| group.get_mut(side))
            .ok_or("unexpected program/side")?;
        assert_eq!(row.ordinal(), *count);
        *count += 1;
        assert_eq!(row.periods().len(), first.sessions().len());
        assert_eq!(row.trades().len() as u64, row.cell().trades);
        assert_eq!(
            row.periods()
                .iter()
                .map(BooleanSessionV1::return_paisa)
                .sum::<i64>(),
            row.cell().pessimistic
        );
        assert_eq!(
            row.periods()
                .iter()
                .map(BooleanSessionV1::trades)
                .sum::<u64>(),
            row.cell().trades
        );
        if row.program_index() == 2 {
            assert_eq!(row.cell().trades, 0);
            assert!(row.summary().unknown > 0);
            assert!(row.selected_exit().is_none());
        }
    }
    assert!(
        groups
            .iter()
            .all(|group| group.iter().all(|count| *count > 0))
    );
    first.require_current()?;
    assert_saved(&fixture, &first)?;
    let second = fixture.produce("NIFTY", &programs)?;
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.completion_digest(), second.completion_digest());
    assert_eq!(first.cohort_digest(), second.cohort_digest());
    assert_eq!(
        first
            .rows()
            .iter()
            .map(BooleanCoordinateV1::identity)
            .collect::<Vec<_>>(),
        second
            .rows()
            .iter()
            .map(BooleanCoordinateV1::identity)
            .collect::<Vec<_>>()
    );
    assert!(first.rows().iter().any(|row| row.cell().trades > 0));
    Ok(())
}

fn assert_saved(fixture: &Fixture, first: &CommittedBooleanFamilyV1) -> Result<(), String> {
    let view = super::reader::Reader::open(&fixture.output, first.identity(), 33_554_432)?;
    assert_eq!(view.programs(), first.programs());
    assert_eq!(view.family(), first.family());
    assert_eq!(view.cohort_digest(), first.cohort_digest());
    assert_eq!(view.coordinate_count(), first.rows().len());
    for (saved, actual) in view.grids().iter().zip(&first.resolutions) {
        assert_eq!(saved.resolution, actual.digest());
        assert_eq!(saved.execution, actual.training_digest());
        assert_eq!(saved.policy, *actual.policy());
        assert_eq!(saved.cells, actual.cell_count());
        assert_eq!(saved.horizon_bars, 5);
        assert_eq!(saved.stops, actual.stop_levels_ppm());
        assert_eq!(saved.targets, actual.target_levels_ppm());
        assert_eq!(saved.trails, actual.trail_levels_ppm());
    }
    let pin = view.completion_digest();
    assert!(view.coordinates([0; 32], 0, 1).is_err());
    assert!(view.coordinates(pin, 0, 257).is_err());
    for (index, actual) in first.rows().iter().enumerate() {
        let saved = view
            .coordinates(pin, index, 1)?
            .into_iter()
            .next()
            .ok_or("missing reader row")?;
        assert_eq!(saved.identity, actual.identity());
        assert_eq!(saved.cell, *actual.cell());
        assert_eq!(saved.support_sessions, actual.support_sessions());
        let trades = view.trades(pin, index, 0, 256)?;
        assert_eq!(
            trades,
            actual
                .trades()
                .get(..actual.trades().len().min(256))
                .ok_or("trade prefix")?
        );
        assert_eq!(
            view.periods(pin, index, 0, 256)?.len(),
            first.sessions().len()
        );
    }
    Ok(())
}

#[test]
fn observer_holds_publication_barrier_and_refuses_changed_owner() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let first = fixture.produce("NIFTY", &programs()?)?;
    let path = first.directory.join("owner.lock");
    let owner = crate::readonly_file::open(&path).map_err(display)?;
    owner.try_lock().map_err(display)?;
    assert!(super::reader::Reader::open(&fixture.output, first.identity(), 33_554_432).is_err());
    // Complete valid bytes alone are insufficient until the publishing lease ends.
    first.require_current()?;
    drop(owner);
    let reader = super::reader::Reader::open(&fixture.output, first.identity(), 33_554_432)?;
    let owner = crate::readonly_file::open(&path).map_err(display)?;
    owner.try_lock().map_err(display)?;
    assert!(
        reader
            .coordinates(reader.completion_digest(), 0, 1)
            .is_err()
    );
    drop(owner);
    let rerun = fixture.produce("NIFTY", &programs()?)?;
    assert_eq!(first.completion_digest(), rerun.completion_digest());
    assert_eq!(
        reader.coordinates(reader.completion_digest(), 0, 1)?.len(),
        1
    );
    let replacement = path.with_extension("replacement");
    fs::write(&replacement, []).map_err(display)?;
    fs::rename(&replacement, &path).map_err(display)?;
    assert!(
        reader
            .coordinates(reader.completion_digest(), 0, 1)
            .is_err()
    );
    drop(reader);
    let reader = super::reader::Reader::open(&fixture.output, first.identity(), 33_554_432)?;
    assert_eq!(reader.coordinate_count(), first.rows().len());
    Ok(())
}

#[test]
fn committed_boolean_family_refuses_body_receipt_and_actual_source_replacement()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let first = fixture.produce("NIFTY", &programs()?)?;
    let body = first.directory.join("body.bin");
    let view = super::reader::Reader::open(&fixture.output, first.identity(), 33_554_432)?;
    let original = fs::read(&body).map_err(display)?;
    let mut corrupt = original.clone();
    *corrupt.last_mut().ok_or("empty body")? ^= 1;
    fs::write(&body, &corrupt).map_err(display)?;
    assert!(first.require_current().is_err());
    assert!(view.coordinates(view.completion_digest(), 0, 1).is_err());
    fs::write(&body, &original).map_err(display)?;
    first.require_current()?;
    let receipt = first.directory.join("complete.bin");
    let original = fs::read(&receipt).map_err(display)?;
    fs::remove_file(&receipt).map_err(display)?;
    assert!(first.require_current().is_err());
    fs::write(&receipt, &original).map_err(display)?;
    first.require_current()?;
    let key = crate::stored::swept_index("NIFTY")?;
    let path = StorePath::for_key(
        brutex_core::vendor::Vendor::Zerodha,
        &key,
        Timeframe::DAY_1,
        YearMonth::new(2025, 4).map_err(display)?,
        FileKind::Bars,
    )
    .map_err(display)?
    .to_path_buf(&fixture.root);
    let bytes = fs::read(&path).map_err(display)?;
    let replacement = path.with_extension("replacement");
    fs::write(&replacement, &bytes).map_err(display)?;
    fs::rename(&replacement, &path).map_err(display)?;
    assert!(
        first.require_current().is_err(),
        "same bytes on a foreign source inode cannot retain strict authority"
    );
    Ok(())
}

#[test]
fn malformed_catalog_and_physical_caps_refuse_without_a_completed_candidate_parent()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    let config = config(&fixture.root)?;
    let long = policy(Side::Long)?;
    let short = policy(Side::Short)?;
    let catalog = programs()?;
    let duplicate = [
        catalog.first().ok_or("program")?.clone(),
        catalog.first().ok_or("program")?.clone(),
    ];
    assert!(
        produce_identified(
            fixture.request("NIFTY", &duplicate, &config, &long, &short)?,
            "generated"
        )
        .is_err()
    );
    assert!(!fixture.output.join("boolean-candidates-v1").exists());
    let mut limited = fixture.request("NIFTY", &catalog, &config, &long, &short)?;
    limited.bounds.bytes = 1;
    assert!(produce_identified(limited, "generated").is_err());
    assert!(!fixture.output.join("boolean-candidates-v1").exists());
    let actual = [Expression::parse("30 | !30").map_err(|why| format!("{why:?}"))?];
    let mut limited = fixture.request("NIFTY", &actual, &config, &long, &short)?;
    limited.bounds.trades = 1;
    let Err(why) = produce_identified(limited, "generated") else {
        return Err("actual priced cell bypassed its trade ceiling".to_owned());
    };
    assert!(why.starts_with("Boolean exact trade evidence exceeds physical ceiling:"));
    assert!(why.contains("program_index=0 side=Long ordinal="));
    let number = |name| -> Result<u64, String> {
        why.split_whitespace()
            .find_map(|word| word.strip_prefix(name))
            .ok_or_else(|| format!("missing physical refusal field {name}"))?
            .parse::<u64>()
            .map_err(display)
    };
    assert!(number("ordinal=")? < long.max_cells());
    assert!(number("needed_trades=")? > 1);
    assert!(number("needed_bytes=")? > number("needed_trades=")? * 88);
    assert_eq!(number("remaining_trades=")?, 1);
    assert!(number("remaining_bytes=")? <= limited.bounds.bytes);
    assert!(!fixture.output.join("boolean-candidates-v1").exists());
    Ok(())
}

#[test]
#[expect(
    clippy::expect_used,
    reason = "one-shot test fault must prove the actual acknowledged file exists before injection"
)]
fn acknowledged_boolean_candidate_body_loss_refuses_before_completion_publication()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = Arc::clone(&called);
    let _fault = CommitFault::install(move |directory| {
        let body = directory.join("body.bin");
        let bytes = fs::read(&body).expect("actual fully written candidate evidence");
        assert!(bytes.len() > 392);
        assert!(!directory.join("complete.bin").exists());
        fs::remove_file(body).expect("inject loss after real acknowledged write");
        seen.store(true, Ordering::SeqCst);
    });
    let result = fixture.produce("NIFTY", &programs()?);
    assert!(result.is_err());
    assert!(called.load(Ordering::SeqCst));
    for entry in fs::read_dir(fixture.output.join("boolean-candidates-v1")).map_err(display)? {
        let path = entry.map_err(display)?.path();
        assert!(!path.join("complete.bin").exists());
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("candidate identity directory")?;
        if name.len() != 64 {
            return Err("candidate identity width".to_owned());
        }
        let mut id = [0; 32];
        for (value, pair) in id.iter_mut().zip(name.as_bytes().chunks_exact(2)) {
            *value = u8::from_str_radix(std::str::from_utf8(pair).map_err(display)?, 16)
                .map_err(display)?;
        }
        let evidence = crate::sweep_evidence::read(&fixture.output, id, 1_048_576)?
            .ok_or("durable attempt missing")?;
        assert_eq!(evidence.completion, Completion::Refused);
    }
    Ok(())
}

#[test]
fn complete_generated_months_measure_actual_accepted_periods_before_stats_fixture_choice()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    let config = config(&fixture.root)?;
    let long = policy(Side::Long)?;
    let short = policy(Side::Short)?;
    let catalog = programs()?;
    let mut counts = Vec::new();
    for month in 5..=8 {
        let mut request = fixture.request("NIFTY", &catalog, &config, &long, &short)?;
        request.from = (2025, month);
        request.to = (2025, month);
        let source = load_source(&request)?;
        let (_, sessions) = prepare_column(&request, &source)?;
        source.guard.require_current()?;
        counts.push((month, sessions.days.len()));
    }
    eprintln!("generated complete month/accepted-session census: {counts:?}");
    assert!(counts.iter().all(|(_, count)| *count > 0));
    let mut request = fixture.request("NIFTY", &catalog, &config, &long, &short)?;
    request.from = (2025, 7);
    request.to = (2025, 8);
    let source = load_source(&request)?;
    let (_, sessions) = prepare_column(&request, &source)?;
    source.guard.require_current()?;
    eprintln!(
        "generated complete July–August accepted-session count: {}",
        sessions.days.len()
    );
    assert_eq!(sessions.days.len(), 42);
    assert!(sessions.days.len().is_multiple_of(2));
    Ok(())
}
