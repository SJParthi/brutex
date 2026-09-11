//! Generated strict-source fixtures; never evidence of market profitability.
use super::*;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;

pub(crate) fn limits() -> Limits {
    Limits {
        programs: 4,
        records: 400_000,
        bytes: 128 * 1024 * 1024,
    }
}
pub(crate) fn source(fixture: &Fixture, symbol: &str, from: u8, to: u8) -> Result<Loaded, String> {
    fixture.prepare(
        symbol
            .strip_prefix("NSE-")
            .ok_or("generated canonical symbol")?,
    )?;
    let strict = StrictConfig::from_values(
        Some(fixture.root.as_os_str().to_owned()),
        Some("134217728".into()),
        Some("400000".into()),
    )
    .map_err(display)?;
    Loaded::load_identified(
        Request {
            store: &fixture.root,
            vendor: "zerodha",
            underlying: symbol,
            rung: "1min",
            from: (2025, from),
            to: (2025, to),
            strict: &strict,
        },
        "generated-index-stop-source-fixture",
    )
}
pub(crate) fn programs() -> Result<Vec<Expression>, String> {
    ["0 | !0", "0 & !0"]
        .iter()
        .map(|value| Expression::parse(value).map_err(|why| format!("{why:?}")))
        .collect()
}

pub(crate) fn load_generated(request: Request<'_>) -> Result<Loaded, String> {
    Loaded::load_identified(request, "generated-index-stop-source-fixture")
}

/// Nine canonical programs over the first live bits, each always-known-true or
/// always-known-false, so the pinned body holds trades and empty readings.
fn pinned_programs() -> Vec<Expression> {
    (0..1024_u32)
        .flat_map(|bit| [format!("{bit} | !{bit}"), format!("{bit} & !{bit}")])
        .filter_map(|text| Expression::parse(&text).ok())
        .take(9)
        .collect()
}

/// This thread's attempt group size until dropped.
struct Group;

impl Group {
    fn of(size: usize) -> Self {
        ATTEMPT_GROUP_OVERRIDE.with(|group| group.set(Some(size)));
        Self
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        ATTEMPT_GROUP_OVERRIDE.with(|group| group.set(None));
    }
}

/// What the ungrouped producer at 759ba45a wrote for `pinned_programs`.
const PINS: [&str; 3] = [
    "35da362a7e9447ce690104094bc4581d63c66ee440cffea0ec72bde465a35fc1",
    "b244ecba43067e3a89a68a9e46f262428089684faa7d17f1f59d6c40efaff63d",
    "8106fa46e8728158036905540dca8ae37e0419454057e909aa560d1e9f888802",
];

fn body_hex(fixture: &Fixture, identity: [u8; 32]) -> Result<String, String> {
    let body = std::fs::read(
        fixture
            .output
            .join(crate::index_stop_store::NAMESPACE)
            .join(crate::identity_hex(&identity))
            .join("body.bin"),
    )
    .map_err(display)?;
    Ok(crate::identity_hex(&hash(&body)))
}

#[test]
fn strict_single_stop_saves_both_directions_and_both_fills_idempotently() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let programs = programs()?;
    let first = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &programs,
        loaded.days(),
        limits(),
    )?;
    assert_eq!(first.evaluations().len(), 4);
    assert_eq!(
        first.evaluations().first().ok_or("long")?.direction(),
        Direction::Long
    );
    assert_eq!(
        first.evaluations().get(1).ok_or("short")?.direction(),
        Direction::Short
    );
    for evaluation in first.evaluations() {
        assert!(evaluation.truth().reconciles());
        assert_eq!(evaluation.policy(), Policy::V1);
        assert!(evaluation.metrics().optimistic_paisa >= evaluation.metrics().pessimistic_paisa);
        assert_eq!(
            evaluation.metrics().trades,
            u64::try_from(evaluation.trades().len()).map_err(display)?
        );
        for trade in evaluation.trades() {
            assert!(trade.optimistic_paisa >= trade.pessimistic_paisa);
            assert_eq!(
                runner::signal_candle_stop::Trade::decode(&trade.canonical_bytes())
                    .map_err(display)?,
                *trade
            );
        }
    }
    assert!(
        first.evaluations().first().ok_or("long")?.metrics().trades > 0,
        "known price condition should produce real fixture executions: {:?}",
        first.evaluations().first().ok_or("long")?.metrics()
    );
    assert_eq!(
        first
            .evaluations()
            .get(2)
            .ok_or("no-trade")?
            .metrics()
            .trades,
        0
    );
    let id = first.identity();
    let pin = first.completion_digest();
    drop(first);
    let second = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &programs,
        loaded.days(),
        limits(),
    )?;
    assert_eq!((second.identity(), second.completion_digest()), (id, pin));
    second.require_current()?;
    Ok(())
}

#[test]
fn index_spot_vwap_unknown_is_not_made_true_by_or_not() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let program = Expression::parse("146 | !146").map_err(|why| format!("{why:?}"))?;
    let saved = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &[program],
        loaded.days(),
        limits(),
    )?;
    for reading in saved.evaluations() {
        assert_eq!(reading.metrics().trades, 0);
        assert_eq!(reading.truth().hits, 0);
        assert!(reading.truth().unknown > 0);
        assert!(reading.truth().reconciles());
    }
    Ok(())
}

#[test]
fn single_stop_retained_vix_publication_is_required_through_outer_terminal() -> Result<(), String> {
    for missing in [false, true] {
        let fixture = Fixture::new()?;
        let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
        let prepared = loaded.prepare(limits())?;
        let committed = produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs()?,
            loaded.days(),
            limits(),
        )?;
        committed.require_current()?;
        let identity = committed.identity();
        let pin = committed.completion_digest();
        let lookup = crate::index_stop_vix::lookup_identity(identity, pin);
        let reference = fixture
            .output
            .join(crate::index_stop_vix::NAMESPACE)
            .join(crate::identity_hex(&lookup));
        let native = fixture
            .output
            .join(crate::index_stop_store::NAMESPACE)
            .join(crate::identity_hex(&identity));
        let before = std::fs::read(native.join("body.bin")).map_err(display)?;
        let owners = [
            crate::readonly_file::open(&native.join("owner.lock")).map_err(display)?,
            crate::readonly_file::open(&reference.join("owner.lock")).map_err(display)?,
        ];
        committed.with_publication_current(|| {
            for owner in &owners {
                assert!(owner.try_lock().is_err());
            }
            Ok(())
        })?;
        for owner in &owners {
            owner.try_lock().map_err(display)?;
            owner.unlock().map_err(display)?;
        }
        // Exercise the same final boundary used by the producer, after the
        // expensive decoded VIX/candidate reader has already been dropped.
        let outer = crate::sweep_evidence::begin(&fixture.output, identity, Operation::IndexStop)?;
        let receipt = reference.join("complete.bin");
        if missing {
            std::fs::remove_file(&receipt).map_err(display)?;
        } else {
            let mut bytes = std::fs::read(&receipt).map_err(display)?;
            *bytes.last_mut().ok_or("generated reference receipt")? ^= 1;
            std::fs::write(&receipt, bytes).map_err(display)?;
        }
        assert!(committed.require_current().is_err());
        assert!(finish_catalog(committed, outer).is_err());
        let latest = crate::sweep_evidence::read(&fixture.output, identity, limits().bytes)?
            .ok_or("outer terminal evidence")?;
        assert_eq!(latest.completion, Completion::Refused);
        assert_eq!(
            std::fs::read(native.join("body.bin")).map_err(display)?,
            before
        );
    }
    Ok(())
}

#[test]
fn single_stop_context_catalog_window_and_capture_admission_are_exact() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let foreign = source(&fixture, "NSE-BANKNIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let other = foreign.prepare(limits())?;
    let programs = programs()?;
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &other,
            &programs,
            loaded.days(),
            limits()
        )
        .is_err()
    );
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &[],
            loaded.days(),
            limits()
        )
        .is_err()
    );
    let duplicate = vec![programs.first().ok_or("program")?.clone(); 2];
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &duplicate,
            loaded.days(),
            limits()
        )
        .is_err()
    );
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            (loaded.days().0 - 1, loaded.days().1),
            limits()
        )
        .is_err()
    );
    let changed = Limits {
        records: limits().records - 1,
        ..limits()
    };
    assert!(
        produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            loaded.days(),
            changed
        )
        .is_err()
    );
    assert!(validate_scope("NSE-RELIANCE", "1min").is_err());
    assert!(validate_scope("NSE-NIFTY", "day").is_err());
    assert!(
        validate_limits(Limits {
            programs: u64::MAX,
            ..limits()
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn later_single_stop_uses_original_first_history_and_a_bound_output_window() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let training = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let later = source(&fixture, "NSE-NIFTY", 5, 6)?;
    let window = (
        i64::from(
            pull::session::Day::new(2025, 6, 1)
                .map_err(display)?
                .days_from_epoch(),
        ),
        later.days().1,
    );
    let prepared = later.prepare(limits())?;
    let programs = programs()?;
    let result = produce_catalog(
        &fixture.output,
        &later,
        &prepared,
        &programs,
        window,
        limits(),
    )?;
    for evaluation in result.evaluations() {
        assert_eq!((evaluation.first_day(), evaluation.last_day()), window);
        assert!(
            evaluation
                .periods()
                .iter()
                .all(|day| day.day >= window.0 && day.day <= window.1)
        );
        assert!(evaluation.trades().iter().all(|trade| {
            (trade.entry_micros + indicators::IST_OFFSET_MICROS).div_euclid(86_400_000_000)
                >= window.0
        }));
    }
    assert!(training.days().1 < window.0);
    assert_eq!(later.days().0, training.days().0);
    assert_ne!(training.source_binding(), later.source_binding());
    Ok(())
}

#[test]
fn deleting_single_stop_receipt_refuses_warm_and_cold_observation() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let prepared = loaded.prepare(limits())?;
    let programs = programs()?;
    let result = produce_catalog(
        &fixture.output,
        &loaded,
        &prepared,
        &programs,
        loaded.days(),
        limits(),
    )?;
    let id = result.identity();
    let complete = fixture
        .output
        .join("index-stop-candidates-v1")
        .join(crate::identity_hex(&id))
        .join("complete.bin");
    std::fs::remove_file(complete).map_err(display)?;
    assert!(result.require_current().is_err());
    assert!(
        crate::index_stop_store::Reader::open(
            &fixture.output,
            id,
            limits().bytes,
            limits().records
        )
        .is_err()
    );
    Ok(())
}

/// Release-only catalog timing and output pins. It prints and never gates: a
/// wall-clock bound fails on a loaded machine without any defect.
#[test]
#[ignore = "timing measurement; run explicitly against a release build"]
fn catalog_attempt_throughput_measurement() -> Result<(), String> {
    let count = std::env::var("BRUTEX_MEASURE_PROGRAMS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(64);
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let _group = std::env::var("BRUTEX_MEASURE_GROUP")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map(Group::of);
    let pinned = pinned_programs();
    let measured = (0..1024_u32)
        .filter_map(|bit| Expression::parse(&format!("{bit} & !{bit}")).ok())
        .take(usize::try_from(count).map_err(display)?)
        .collect::<Vec<_>>();
    for (label, programs) in [("pin", pinned), ("throughput", measured)] {
        let limits = Limits {
            programs: u64::try_from(programs.len()).map_err(display)?,
            ..limits()
        };
        let prepared = loaded.prepare(limits)?;
        let before = crate::sweep_evidence::flush_count();
        let started = std::time::Instant::now();
        let saved = produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            loaded.days(),
            limits,
        )?;
        let micros = started.elapsed().as_micros().max(1);
        let flushes = u128::from(crate::sweep_evidence::flush_count() - before);
        let attempts = u128::try_from(programs.len() * 2 + 1).map_err(display)?;
        println!(
            "{label}: group={} attempts={attempts} micros={micros} milli_attempts_per_s={} flushes={flushes} milli_flushes_per_attempt={} identity={} completion={} body={}",
            attempt_group(),
            attempts * 1_000_000_000 / micros,
            flushes * 1000 / attempts,
            crate::identity_hex(&saved.identity()),
            crate::identity_hex(&saved.completion_digest()),
            body_hex(&fixture, saved.identity())?,
        );
    }
    Ok(())
}

/// Catalog bytes, digests and every per-run terminal are what the ungrouped
/// producer wrote at 759ba45a: for the production group, for one, and for a
/// group of four that leaves a partial tail.
#[test]
fn grouped_catalogs_keep_the_ungrouped_bytes_and_terminals() -> Result<(), String> {
    let programs = pinned_programs();
    let limits = Limits {
        programs: 9,
        ..limits()
    };
    for size in [None, Some(1), Some(4)] {
        let fixture = Fixture::new()?;
        let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
        let prepared = loaded.prepare(limits)?;
        let _group = size.map(Group::of);
        let saved = produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            loaded.days(),
            limits,
        )?;
        let found = [
            crate::identity_hex(&saved.identity()),
            crate::identity_hex(&saved.completion_digest()),
            body_hex(&fixture, saved.identity())?,
        ];
        assert_eq!(found, PINS, "group {size:?}");
        assert_eq!(saved.evaluations().len(), 18);
        for evaluation in saved.evaluations() {
            let run =
                crate::sweep_evidence::read(&fixture.output, evaluation.run_id(), limits.bytes)?
                    .map(|run| (run.operation, run.completion));
            assert_eq!(run, Some((Operation::IndexStop, Completion::Completed)));
        }
    }
    Ok(())
}

/// The grouped loop's own refusals: a run identity the evaluator does not
/// reproduce, and observed days beyond the aggregate record admission.
#[test]
fn grouped_evaluation_refuses_a_foreign_identity_and_excess_records() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    // Always known false: no events, yet every observed day is a record.
    let never: Vec<Expression> = pinned_programs().into_iter().skip(1).take(1).collect();
    let _quiet = crate::sweep_evidence::tests::BarrierWatch::modelled(|_| Ok(()));
    let one = Limits {
        programs: 1,
        ..limits()
    };
    let prepared = loaded.prepare(one)?;
    let mut attempts = Vec::new();
    let foreign = evaluate_grouped(
        &fixture.output,
        &prepared.native,
        &never,
        &[[7; 32]; 2],
        loaded.days(),
        one,
        &mut attempts,
    )
    .err()
    .unwrap_or_default();
    assert_eq!(
        foreign,
        "single-stop execution identity differs from its durable start"
    );
    assert_eq!(
        attempts.len(),
        2,
        "both identities were durably begun first"
    );
    drop(attempts);
    let tight = Limits {
        programs: 1,
        records: 2,
        ..limits()
    };
    let prepared = loaded.prepare(tight)?;
    let runs = run_ids(&prepared.native, &never, loaded.days())?;
    let mut attempts = Vec::new();
    let excess = evaluate_grouped(
        &fixture.output,
        &prepared.native,
        &never,
        &runs,
        loaded.days(),
        tight,
        &mut attempts,
    )
    .err()
    .unwrap_or_default();
    assert_eq!(
        excess,
        "single-stop complete evidence exceeds aggregate record admission"
    );
    Ok(())
}

/// A start refused in a later group, before anything in it evaluates, is the
/// refusal returned; a refused finish drops every unfinished run as Refused.
/// Neither leaves a completed run or catalog.
#[test]
fn refused_later_starts_and_finishes_leave_no_completion() -> Result<(), String> {
    let programs = pinned_programs();
    let limits = Limits {
        programs: 9,
        ..limits()
    };
    // Identity rows: the catalog's, then four per group. Lifecycle rows: the
    // catalog's and eighteen run starts, then the first run's terminal.
    for (suffix, at) in [("starts.bin", 6_u32), ("-lifecycle.bin", 20)] {
        let fixture = Fixture::new()?;
        let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
        let prepared = loaded.prepare(limits)?;
        let _group = Group::of(4);
        let counter = std::rc::Rc::new(std::cell::Cell::new(0_u32));
        let seen = std::rc::Rc::clone(&counter);
        let watch = crate::sweep_evidence::tests::BarrierWatch::modelled(move |path| {
            if path.to_string_lossy().ends_with(suffix) {
                seen.set(seen.get() + 1);
                if seen.get() == at {
                    return Err(std::io::Error::other("injected refusal"));
                }
            }
            Ok(())
        });
        let refusal = produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            loaded.days(),
            limits,
        )
        .err()
        .unwrap_or_default();
        drop(watch);
        assert!(refusal.contains("injected refusal"), "{suffix}: {refusal}");
        let (catalog, runs) =
            catalog_identity(&loaded, &prepared.native, &programs, loaded.days(), limits)?;
        for identity in runs.iter().chain([&catalog]) {
            let saved = crate::sweep_evidence::read(&fixture.output, *identity, limits.bytes)?;
            assert_ne!(
                saved.map(|saved| saved.completion),
                Some(Completion::Completed),
                "{suffix}"
            );
        }
    }
    Ok(())
}

/// A start refused inside a group still returns the refusal sequential starts
/// meet first: here the first evaluation's, not the later start's.
#[test]
fn a_refused_start_inside_a_group_keeps_the_sequential_first_refusal() -> Result<(), String> {
    let programs = pinned_programs();
    let limits = Limits {
        programs: 9,
        records: 18,
        ..limits()
    };
    let refusal = |group: usize, refuse_start: Option<u32>| -> Result<String, String> {
        let fixture = Fixture::new()?;
        let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
        let prepared = loaded.prepare(limits)?;
        let _group = Group::of(group);
        let counter = std::rc::Rc::new(std::cell::Cell::new(0_u32));
        let starts = std::rc::Rc::clone(&counter);
        let _watch = crate::sweep_evidence::tests::BarrierWatch::modelled(move |path| {
            if path.ends_with("starts.bin") {
                starts.set(starts.get() + 1);
                if Some(starts.get()) == refuse_start {
                    return Err(std::io::Error::other("injected start refusal"));
                }
            }
            Ok(())
        });
        let outcome = produce_catalog(
            &fixture.output,
            &loaded,
            &prepared,
            &programs,
            loaded.days(),
            limits,
        );
        Ok(outcome.err().unwrap_or_default())
    };
    let sequential = refusal(1, None)?;
    assert!(
        !sequential.is_empty() && !sequential.contains("injected"),
        "{sequential}"
    );
    assert_eq!(refusal(16, None)?, sequential);
    // The catalog's own start writes the first identity row; the fourth is the
    // third run of the first group, refused after two runs started durably.
    assert_eq!(refusal(16, Some(4))?, sequential);
    Ok(())
}
