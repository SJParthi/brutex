#![cfg(test)]
//! Generated complete-calendar source authority through the actual search command.
//! This proves finite recovery and observation contracts, not market performance.
use super::*;
use crate::boolean_campaign::tests::generated_prepared;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;
use crate::search_checkpoint::{Saved, Snapshot, tests::Scratch};
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

const CHILD: &str = "BRUTEX_QUALIFIED_SEARCH_GENERATED_CHILD";
const TEST: &str = "boolean_search_command::integration_tests::generated_search_recovers_same_ordinal_and_refuses_missing_ancestry";
const BYTES: u64 = 64 * 1024 * 1024;
const RECORDS: u64 = 3_000_000;
const INJECTED: &str = "generated admitted-source factory refuses before qualification work";
thread_local! {
    static FAULT: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

struct Fault;
impl Fault {
    fn at(call: usize) -> Self {
        FAULT.set((0, call));
        Self
    }
}
impl Drop for Fault {
    fn drop(&mut self) {
        FAULT.set((0, 0));
    }
}

fn generated_input(input: Input<'_>, out: &mut String) -> Result<Prepared, String> {
    let mut prepared = Prepared::new(input, out)?;
    prepared.generated_fixture()?;
    Ok(prepared)
}
fn admitted_factory(request: &campaign::Request<'_>, out: &mut String) -> Result<Campaign, String> {
    let prepared = generated_prepared(request, out)?;
    prepared.require_current()?;
    let (old, fail) = FAULT.get();
    let call = old
        .checked_add(1)
        .ok_or("generated call counter overflow")?;
    FAULT.set((call, fail));
    if call == fail {
        return Err(INJECTED.into());
    }
    Ok(prepared)
}

#[test]
fn generated_search_recovers_same_ordinal_and_refuses_missing_ancestry() -> Result<(), String> {
    if let Some(root) = std::env::var_os(CHILD) {
        if std::env::var("BRUTEX_SELECTED_SEARCH_FIXTURE").as_deref() == Ok("yes") {
            return actual_selected(&PathBuf::from(root));
        }
        return actual(&PathBuf::from(root));
    }
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    campaign::tests::coarse_fixture(&fixture);
    generated_october(&fixture.root)?;
    for month in [9, 10] {
        later_coarse(&fixture.root, month)?;
    }
    run_fixture_child(&fixture, false)
}

#[test]
fn generated_selected_search_reads_no_unselected_signal_rungs_and_resumes_exact_scope()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    generated_october(&fixture.root)?;
    // This fixture contains execution minutes and daily references only.
    // Any accidental coarse signal-rung source preparation must refuse.
    run_fixture_child(&fixture, true)
}

fn run_fixture_child(fixture: &Fixture, selected: bool) -> Result<(), String> {
    let log = fixture.root.join("generated-search-child.log");
    let file = fs::File::create_new(&log).map_err(display)?;
    let mut command = std::process::Command::new(std::env::current_exe().map_err(display)?);
    command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]);
    command.env_clear();
    // Instrumented complete-calendar replay exceeded the ordinary deadline on
    // the Linux CI runner. Keep the same assertions and finite log bound.
    let timeout = if std::env::var_os("LLVM_PROFILE_FILE").is_some() {
        std::time::Duration::from_secs(900)
    } else {
        std::time::Duration::from_secs(360)
    };
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let mut child = command
        .env(CHILD, &fixture.root)
        .env(
            "BRUTEX_SELECTED_SEARCH_FIXTURE",
            if selected { "yes" } else { "no" },
        )
        .env("BRUTEX_STORE", &fixture.root)
        .env("BRUTEX_CHECKSUM_RECEIPTS", &fixture.root)
        .env("BRUTEX_CHECKSUM_MAX_BYTES", BYTES.to_string())
        .env("BRUTEX_CHECKSUM_MAX_RECORDS", RECORDS.to_string())
        .env(
            "BRUTEX_ADMISSION_POLICY_FILE",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/intraday-research-v1.toml"),
        )
        .stdout(file.try_clone().map_err(display)?)
        .stderr(file)
        .spawn()
        .map_err(display)?;
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(display)? {
            break status;
        }
        let log_bytes = fs::metadata(&log).map_err(display)?.len();
        if started.elapsed() > timeout || log_bytes > 8 * 1024 * 1024 {
            child.kill().map_err(display)?;
            child.wait().map_err(display)?;
            let detail = if fs::metadata(&log).map_err(display)?.len() <= 8 * 1024 * 1024 {
                fs::read_to_string(&log).map_err(display)?
            } else {
                "child log exceeded 8MiB; refusing to load it".into()
            };
            return Err(format!(
                "generated search child exceeded its {}s/8MiB bound after {:.1}s ({log_bytes} bytes): {detail}",
                timeout.as_secs(),
                started.elapsed().as_secs_f64()
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    if fs::metadata(&log).map_err(display)?.len() > 8 * 1024 * 1024 {
        return Err("generated search child exceeded its 8MiB log bound".into());
    }
    let output = fs::read_to_string(&log).map_err(display)?;
    assert!(status.success(), "{output}");
    assert!(output.contains("1 passed"), "{output}");
    Ok(())
}

fn actual_selected(root: &Path) -> Result<(), String> {
    let output = root.join("qualified-selected-search-output");
    fs::create_dir(&output).map_err(display)?;
    let mut request = parse(&[
        "zerodha",
        "NIFTY",
        "2025",
        "7",
        "2025",
        "8",
        "146",
        "5",
        "50",
        "1",
        "100",
        "1",
        output.to_str().ok_or("generated output UTF-8")?,
        "2025",
        "9",
        "2025",
        "10",
    ])?;
    request.rungs = campaign::RungScope::new(&["1min"])?;
    let mut out = String::new();
    reserve_and_observe_failure(&request, &mut out)?;
    let identity = only_identity(&output)?;
    let pending = Reader::open(&output, identity, BYTES * 4, RECORDS)?;
    assert_eq!(pending.rungs(), request.rungs);
    assert_eq!(pending.projection_version(), 4);
    assert_eq!(pending.completed_batches(), 0);
    let first = snapshot(&output, identity)?.read(1, BYTES)?;
    drop(pending);
    resume_and_observe(&request, &mut out, identity)?;
    let reader = Reader::open(&output, identity, BYTES * 4, RECORDS)?;
    assert_eq!(reader.completed_batches(), 1);
    assert_eq!(reader.rungs().labels(), ["1min"]);
    assert!(
        !reader.exhausted(),
        "one batch allowance is not grammar exhaustion"
    );
    assert!(reader.summaries()[0].count > 0);
    for index in 1..8 {
        assert_eq!(reader.summaries().get(index), Some(&Summary::EMPTY));
        assert!(reader.rung(reader.pin(), 0, index).is_err());
    }
    reader.verify_batch(0)?;
    same_record(&first, &snapshot(&output, identity)?.read(1, BYTES)?);
    let saved = reader.rung(reader.pin(), 0, 0)?;
    assert_eq!(saved.projection_version(), 4);
    assert_eq!(
        saved.source().allocation().rungs(),
        8,
        "unselected alpha units stay unspent"
    );
    let campaign = crate::boolean_evidence::QualifiedCampaign::open(
        &output,
        reader
            .planned_campaign()?
            .ok_or("selected planned campaign")?,
        BYTES * 4,
    )?;
    assert_eq!(campaign.rungs(), request.rungs);
    assert!(campaign.slots()[0].complete.is_some());
    for slot in campaign.slots().iter().skip(1) {
        assert!(!slot.started);
        assert!(slot.complete.is_none());
        assert!(slot.reason.is_empty());
    }
    let pin = reader.pin();
    // Widening the same input to all eight must touch the absent coarse series
    // and refuse, proving the successful subset did not require those inputs.
    request.rungs = campaign::RungScope::ALL;
    assert!(
        request
            .prepare(
                &[runner::expression::Expression::parse("146").map_err(debug)?],
                false,
                &mut String::new(),
                admitted_factory
            )
            .is_err()
    );
    reader.require_current()?;
    assert_eq!(
        Reader::open(&output, identity, BYTES * 4, RECORDS)?.pin(),
        pin
    );
    assert!(out.contains("unselected units are unspent"));
    Ok(())
}

fn later_coarse(root: &Path, month: u8) -> Result<(), String> {
    use store::path::{FileKind, StorePath, Timeframe, YearMonth};
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let key = crate::stored::swept_index("NIFTY")?;
    let loaded = crate::stored::load(root, vendor, "NIFTY", "1min", 2025, month)?;
    let minutes: Vec<_> = loaded
        .bars
        .iter()
        .map(|bar| store::format::Bar {
            ts_micros: bar.ts_micros,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: bar.open_interest,
        })
        .collect();
    let symbol = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let symbol = u32::from_le_bytes(
        symbol
            .get(..4)
            .ok_or("symbol prefix")?
            .try_into()
            .map_err(|_| "symbol width")?,
    );
    for seconds in [120, 180, 300, 600, 900, 1800, 3600] {
        let rows = pull::fold::fold_from_bars(
            &minutes,
            pull::fold::Bucket::of_secs(seconds).ok_or("generated bucket")?,
            pull::fold::Bucket::MINUTE,
        )
        .map_err(display)?;
        let path = StorePath::for_key(
            vendor,
            &key,
            Timeframe::from_secs(seconds).map_err(display)?,
            YearMonth::new(2025, month).map_err(display)?,
            FileKind::Bars,
        )
        .map_err(display)?;
        let mut file = store::file::BarFile::open_or_create(root, path, symbol).map_err(display)?;
        file.append(&rows).map_err(display)?;
    }
    Ok(())
}

fn generated_october(root: &Path) -> Result<(), String> {
    use store::format::Bar;
    use store::path::{FileKind, StorePath, Timeframe, YearMonth};
    let mut minutes = Vec::new();
    let mut daily = Vec::new();
    for day in 1..=31 {
        let date = pull::session::Day::new(2025, 10, day).map_err(display)?;
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
        let rows = minutes
            .get(start..)
            .ok_or("generated October session slice")?;
        let first = rows.first().ok_or("generated October empty session")?;
        daily.push(Bar {
            ts_micros: first.ts_micros,
            open: first.open,
            high: rows
                .iter()
                .map(|row| row.high)
                .max()
                .ok_or("generated high")?,
            low: rows
                .iter()
                .map(|row| row.low)
                .min()
                .ok_or("generated low")?,
            close: rows.last().ok_or("generated close")?.close,
            volume: rows.iter().map(|row| row.volume).sum(),
            open_interest: i64::MIN,
        });
    }
    let symbol = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let symbol = u32::from_le_bytes(
        symbol
            .get(..4)
            .ok_or("symbol prefix")?
            .try_into()
            .map_err(|_| "symbol width")?,
    );
    let key = crate::stored::swept_index("NIFTY")?;
    for (rung, rows) in [(Timeframe::MINUTE_1, minutes), (Timeframe::DAY_1, daily)] {
        let path = StorePath::for_key(
            brutex_core::vendor::Vendor::Zerodha,
            &key,
            rung,
            YearMonth::new(2025, 10).map_err(display)?,
            FileKind::Bars,
        )
        .map_err(display)?;
        let mut file = store::file::BarFile::open_or_create(root, path, symbol).map_err(display)?;
        file.append(&rows).map_err(display)?;
    }
    Ok(())
}

fn reserve_and_observe_failure(request: &Request<'_>, out: &mut String) -> Result<(), String> {
    let _fault = Fault::at(6);
    let expected = launch::fingerprint(&generated_input(request.input, &mut String::new())?);
    let mut changed = expected;
    *changed.first_mut().ok_or("fingerprint byte missing")? ^= 1;
    let mut prematurely_published = false;
    let refused = execute_observed(
        request,
        &mut String::new(),
        generated_input,
        admitted_factory,
        Some(changed),
        &mut |_| {
            prematurely_published = true;
            Ok(())
        },
    )
    .err()
    .ok_or("changed launch policy was not refused")?;
    assert!(refused.contains("configuration changed after admission"));
    assert_eq!(FAULT.get(), (0, 6));
    assert!(!prematurely_published);
    assert!(!request.input.output.join(NAMESPACE).exists());
    let mut published = 0;
    let why = execute_observed(
        request,
        out,
        generated_input,
        admitted_factory,
        Some(expected),
        &mut |event| {
            let readable = Reader::open(request.input.output, event.identity, BYTES * 4, RECORDS)?;
            assert_eq!(readable.completed_batches(), 0);
            assert_eq!(event.exhausted, None);
            assert_eq!(event.completed_batches, None);
            published += 1;
            Ok(())
        },
    )
    .err()
    .ok_or("generated pre-work refusal was not invoked")?;
    assert_eq!(
        published, 1,
        "only a durably readable reservation publishes an identity"
    );
    assert!(why.contains(INJECTED), "{why}");
    assert!(why.contains("refusal recorded"), "{why}");
    assert_eq!(FAULT.get(), (6, 6));
    Ok(())
}

fn resume_and_observe(
    request: &Request<'_>,
    out: &mut String,
    identity: [u8; 32],
) -> Result<(), String> {
    let mut observed = Vec::new();
    let expected = launch::fingerprint(&generated_input(request.input, &mut String::new())?);
    execute_observed(
        request,
        out,
        generated_input,
        admitted_factory,
        Some(expected),
        &mut |event| {
            let readable = Reader::open(request.input.output, event.identity, BYTES * 4, RECORDS)?;
            if let Some(exhausted) = event.exhausted {
                assert_eq!(exhausted, readable.exhausted());
                assert_eq!(
                    event.completed_batches,
                    Some(readable.completed_batches() as u64)
                );
            }
            observed.push(event);
            Ok(())
        },
    )?;
    assert!(observed.len() >= 2);
    assert!(observed.iter().all(|event| event.identity == identity));
    assert_eq!(
        observed.last().and_then(|event| event.exhausted),
        Some(false)
    );
    Ok(())
}

fn actual(root: &Path) -> Result<(), String> {
    let output = root.join("qualified-search-output");
    fs::create_dir(&output).map_err(display)?;
    let request = parse(&[
        "zerodha",
        "NIFTY",
        "2025",
        "7",
        "2025",
        "8",
        "146",
        "5",
        "50",
        "1",
        "100",
        "1",
        output.to_str().ok_or("generated output UTF-8")?,
        "2025",
        "9",
        "2025",
        "10",
    ])?;
    let mut out = String::new();
    reserve_and_observe_failure(&request, &mut out)?;
    let identity = only_identity(&output)?;
    let refused = snapshot(&output, identity)?;
    assert_eq!((refused.latest, refused.acknowledged), (Some(2), 2));
    let reservation = refused.read(1, BYTES)?;
    let failed = refused.read(2, BYTES)?;
    let first = Record::decode(&reservation.payload, BYTES)?;
    let failure = Record::decode(&failed.payload, BYTES)?;
    assert_eq!(
        (first.ordinal, first.phase, failure.ordinal, failure.phase),
        (0, 0, 0, 2)
    );
    assert_eq!(first.binding()?, failure.binding()?);
    assert_eq!(failure.reason, INJECTED);
    let pending = Reader::open(&output, identity, BYTES * 4, RECORDS)?;
    assert_eq!(pending.completed_batches(), 0);
    let planned_campaign = pending
        .planned_campaign()?
        .ok_or("planned campaign absent")?;
    drop(pending);

    out.clear();
    resume_and_observe(&request, &mut out, identity)?;
    assert!(out.contains("PAUSED AT INVOCATION WORK ALLOWANCE"), "{out}");
    let complete = snapshot(&output, identity)?;
    assert_eq!((complete.latest, complete.acknowledged), (Some(3), 3));
    same_record(&reservation, &complete.read(1, BYTES)?);
    same_record(&failed, &complete.read(2, BYTES)?);
    let saved = complete.read(3, BYTES)?;
    let completed = Record::decode(&saved.payload, BYTES)?;
    assert_eq!((completed.ordinal, completed.phase), (0, 1));
    assert_eq!(completed.campaign.identity, planned_campaign);
    assert_eq!(first.binding()?, completed.binding()?);
    assert_eq!(completed.batch.programs(), first.batch.programs());
    assert!(
        completed
            .summaries
            .iter()
            .all(|s| s.count > 0 && s.child.pin != [0; 32])
    );
    let reader = Reader::open(&output, identity, BYTES * 4, RECORDS)?;
    assert_eq!(
        (reader.completed_batches(), reader.batch(), reader.phase()),
        (1, 0, 1)
    );
    reader.verify_batch(0)?;
    legacy_saved_children_keep_their_recorded_projection_rule(&output, &completed)?;
    reader.require_current()?;
    same_record(&saved, &snapshot(&output, identity)?.read(3, BYTES)?);
    prepublication_accounting(&output, &completed)?;
    retained_page_refuses_parent_loss(root, &output, &reader)?;
    missing_child_blocks_restart(root, &request, identity, &saved, &completed)?;
    Ok(())
}

fn legacy_saved_children_keep_their_recorded_projection_rule(
    output: &Path,
    completed: &Record,
) -> Result<(), String> {
    use crate::boolean_search_record::ProjectionRule;

    assert_eq!(
        completed.spec.projection_rule,
        ProjectionRule::IndexConsistencyV4
    );
    // Generated compatibility records retain actual generated children. Neither
    // repricing nor rewriting the original V4 journal is needed to observe V1.
    let mut legacy = Record::decode(&completed.encode()?, BYTES)?;
    legacy.spec.projection_rule = ProjectionRule::LegacyV1;
    for (rung, summary) in legacy.summaries.iter_mut().enumerate() {
        let source = crate::boolean_evidence::Qualification::open(
            output,
            summary.child.identity,
            BYTES * 4,
        )?;
        let allocation =
            runner::search_allocation_v1::allocate(legacy.ordinal, rung as u64, legacy.spec.alpha)
                .map_err(debug)?;
        let old = *summary;
        *summary = crate::boolean_search_projection::summarize(
            &source,
            allocation,
            ProjectionRule::LegacyV1,
        )?;
        assert_eq!(
            (summary.child, summary.count, summary.counts),
            (old.child, old.count, old.counts)
        );
        assert_ne!(
            summary.projection, old.projection,
            "versioned domains differ"
        );
    }
    let pin = publish_generated_compatibility_history(output, &mut legacy)?;
    assert_ne!(legacy.spec.identity(), completed.spec.identity());
    let reader = Reader::open(output, legacy.spec.identity(), BYTES * 4, RECORDS)?;
    assert_eq!((reader.projection_version(), reader.pin()), (1, pin));
    assert_eq!(reader.completed_batches(), 1);
    assert_eq!(reader.summaries(), &legacy.summaries);
    reader.verify_batch(0)?;
    for rung in 0..8 {
        let selected = reader.rung(pin, 0, rung)?;
        assert_eq!(selected.projection_version(), 1);
        let original_policy = selected.source().policy().digest();
        assert_eq!(selected.policy().digest(), original_policy);
        let rows = selected.rows(selected.source().completion_digest(), 0, 1)?;
        assert_eq!(rows.len(), 1, "actual generated child page is nonempty");
        assert!(
            rows.iter()
                .all(|row| row.projection.policy_digest() == original_policy)
        );
        selected.require_current()?;
    }
    legacy_saved_child_refuses_a_resealed_current_projection(output, &legacy, completed)?;
    reader.require_current()?;
    assert_eq!(reader.pin(), pin);
    Ok(())
}

fn publish_generated_compatibility_history(
    output: &Path,
    completed: &mut Record,
) -> Result<[u8; 32], String> {
    let mut reserved = Record::decode(&completed.encode()?, BYTES)?;
    reserved.previous = None;
    reserved.phase = 0;
    reserved.campaign = Summary::EMPTY.child;
    reserved.summaries = [Summary::EMPTY; 8];
    reserved.reason.clear();
    transition(None, &reserved)?;
    let mut journal = Journal::open(output, NAMESPACE, reserved.spec.identity())?;
    assert!(
        journal.latest(BYTES)?.is_none(),
        "separate generated compatibility identity"
    );
    completed.previous = Some(journal.publish(&reserved.encode()?, BYTES)?);
    transition(Some(&reserved), completed)?;
    Ok(journal.publish(&completed.encode()?, BYTES)?.1)
}

fn legacy_saved_child_refuses_a_resealed_current_projection(
    output: &Path,
    legacy: &Record,
    current: &Record,
) -> Result<(), String> {
    let mut mixed = Record::decode(&legacy.encode()?, BYTES)?;
    // An independently declared generated replay allowance yields a new journal
    // address, while this first batch still names the exact same source children.
    mixed.spec.nodes = mixed
        .spec
        .nodes
        .checked_add(1)
        .ok_or("fixture node overflow")?;
    mixed.batch = Batch::prepare(mixed.spec.cursor()?, 0, 0, mixed.spec.budget())?;
    assert_eq!(mixed.batch.programs(), legacy.batch.programs());
    *mixed.summaries.first_mut().ok_or("legacy first summary")? =
        *current.summaries.first().ok_or("current first summary")?;
    assert_ne!(mixed.spec.identity(), legacy.spec.identity());
    let pin = publish_generated_compatibility_history(output, &mut mixed)?;
    let reader = Reader::open(output, mixed.spec.identity(), BYTES * 4, RECORDS)?;
    assert_eq!((reader.projection_version(), reader.pin()), (1, pin));
    assert_eq!(
        reader.completed_batches(),
        1,
        "the full record is structurally valid"
    );
    let expected = "search complete projection, allocation or child plan differs";
    assert_eq!(reader.verify_batch(0).err().as_deref(), Some(expected));
    assert_eq!(reader.rung(pin, 0, 0).err().as_deref(), Some(expected));
    reader.require_current()?;
    Ok(())
}

fn prepublication_accounting(output: &Path, completed: &Record) -> Result<(), String> {
    // Pre-publication reconciliation also uses this internal child-only reader.
    // Its accounting must describe that exact retained scope, not invent a parent.
    let standalone = crate::boolean_search_reader::open_rung(output, completed, 0, BYTES * 4)?;
    standalone.require_current()?;
    assert_eq!(
        standalone.admitted_bytes()?,
        standalone.source().admitted_bytes()
    );
    assert_eq!(standalone.replay_nodes()?, completed.spec.nodes);
    assert_eq!(
        standalone.summary(),
        *completed
            .summaries
            .first()
            .ok_or("first complete summary")?
    );
    Ok(())
}

fn missing_child_blocks_restart(
    root: &Path,
    request: &Request<'_>,
    identity: [u8; 32],
    saved: &Saved,
    completed: &Record,
) -> Result<(), String> {
    let output = request.input.output;
    let child = completed
        .summaries
        .first()
        .ok_or("first rung absent")?
        .child;
    let receipt = output
        .join("boolean-qualification-v1")
        .join(crate::identity_hex(&child.identity))
        .join("complete.bin");
    let original = fs::read(&receipt).map_err(display)?;
    assert_eq!(
        original.len(),
        112,
        "an actual acknowledged child exists before removal"
    );
    let moved = Moved::new(&receipt, &root.join("held-qualification-completion"))?;
    let mut out = String::new();
    let why = execute_with(request, &mut out, generated_input, admitted_factory)
        .err()
        .ok_or("missing prior child was accepted on restart")?;
    assert!(
        !why.contains(INJECTED),
        "restart must fail on real missing ancestry: {why}"
    );
    let unchanged = snapshot(output, identity)?;
    assert_eq!((unchanged.latest, unchanged.acknowledged), (Some(3), 3));
    same_record(saved, &unchanged.read(3, BYTES)?);
    moved.restore()?;
    assert_eq!(fs::read(&receipt).map_err(display)?, original);
    Reader::open(output, identity, BYTES * 4, RECORDS)?.verify_batch(0)?;

    // Restoring the exact child permits the genuine next reservation. Stop at
    // its source-admitted work boundary instead of pricing a second population.
    let _fault = Fault::at(6);
    let why = execute_with(request, &mut out, generated_input, admitted_factory)
        .err()
        .ok_or("next-batch pre-work fault was not invoked")?;
    assert!(
        why.contains(INJECTED) && why.contains("refusal recorded"),
        "{why}"
    );
    assert_eq!(FAULT.get(), (6, 6));
    let resumed = snapshot(output, identity)?;
    assert_eq!((resumed.latest, resumed.acknowledged), (Some(5), 5));
    same_record(saved, &resumed.read(3, BYTES)?);
    let pending = Record::decode(&resumed.read(4, BYTES)?.payload, BYTES)?;
    let failed = Record::decode(&resumed.read(5, BYTES)?.payload, BYTES)?;
    assert_eq!(
        (pending.ordinal, pending.phase, failed.ordinal, failed.phase),
        (1, 0, 1, 2)
    );
    assert_eq!(pending.binding()?, failed.binding()?);
    assert!(pending.batch.follows(
        &completed.batch.next_cursor(),
        completed.batch.work(),
        completed.batch.cumulative_programs()?
    ));
    assert_ne!(
        campaign::program_digest(pending.batch.programs()),
        campaign::program_digest(completed.batch.programs())
    );
    let reader = Reader::open(output, identity, BYTES * 4, RECORDS)?;
    assert_eq!(
        (reader.completed_batches(), reader.batch(), reader.phase()),
        (1, 1, 2)
    );
    reader.verify_batch(0)?;
    Ok(())
}

fn retained_page_refuses_parent_loss(
    root: &Path,
    output: &Path,
    reader: &Reader,
) -> Result<(), String> {
    let rung = reader.rung(reader.pin(), 0, 0)?;
    let pin = rung.source().completion_digest();
    let expected = *reader.summaries().first().ok_or("first search summary")?;
    assert_eq!(rung.summary(), expected);
    assert_eq!(rung.source().identity(), expected.child.identity);
    assert_eq!(pin, expected.child.pin);
    assert_eq!(rung.source().row_count() as u64, expected.count);
    let allocation = runner::search_allocation_v1::allocate(0, 0, reader.alpha_ppm())
        .map_err(|why| format!("{why:?}"))?;
    assert_eq!(rung.allocation(), allocation);
    assert_eq!(rung.allocation().digest(), expected.allocation);
    let campaign = crate::boolean_evidence::QualifiedCampaign::open(
        output,
        reader
            .planned_campaign()?
            .ok_or("complete campaign address")?,
        BYTES * 4,
    )?;
    assert_eq!(
        rung.admitted_bytes()?,
        reader.admitted_bytes() + campaign.admitted_bytes() + rung.source().admitted_bytes()
    );
    assert_eq!(
        rung.replay_nodes()?,
        reader.replay_nodes() + reader.spec().nodes
    );
    assert_eq!(rung.rows(pin, 0, 1)?.len(), 1);
    assert!(rung.rows([0; 32], 0, 1).is_err());
    for invalid in [8, usize::MAX] {
        assert_eq!(
            reader.rung(reader.pin(), 0, invalid).err(),
            Some("search timeframe outside declared eight".into())
        );
    }
    let parent = checkpoint_payload(output, reader.identity(), reader.sequence());
    let moved = Moved::new(&parent, &root.join("held-search-parent"))?;
    assert!(
        rung.require_current().is_err(),
        "retained child lost its actual parent"
    );
    assert!(
        rung.rows(pin, 0, 1).is_err(),
        "no child-only page may escape after parent loss"
    );
    moved.restore()?;
    rung.require_current()?;
    assert_eq!(rung.rows(pin, 0, 1)?.len(), 1);
    Ok(())
}

fn checkpoint_payload(root: &Path, identity: [u8; 32], sequence: u64) -> PathBuf {
    root.join(NAMESPACE)
        .join(crate::identity_hex(&identity))
        .join(format!("{sequence:016x}"))
        .join("payload")
}
fn snapshot(root: &Path, identity: [u8; 32]) -> Result<Snapshot, String> {
    Snapshot::open(root, NAMESPACE, identity)?.ok_or_else(|| "search snapshot absent".into())
}
fn same_record(before: &Saved, after: &Saved) {
    assert_eq!(
        (before.sequence, before.seal, &before.payload),
        (after.sequence, after.seal, &after.payload)
    );
}
fn only_identity(root: &Path) -> Result<[u8; 32], String> {
    let mut entries = fs::read_dir(root.join(NAMESPACE)).map_err(display)?;
    let path = entries
        .next()
        .ok_or("search identity absent")?
        .map_err(display)?
        .path();
    assert!(entries.next().is_none(), "one immutable search scope");
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("identity UTF-8")?;
    if name.len() != 64 {
        return Err("identity width".into());
    }
    let decoded = name
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(display)?;
            u8::from_str_radix(text, 16).map_err(display)
        })
        .collect::<Result<Vec<_>, _>>()?;
    decoded.try_into().map_err(|_| "identity byte width".into())
}

struct Moved {
    original: PathBuf,
    held: PathBuf,
    active: bool,
}
impl Moved {
    fn new(original: &Path, held: &Path) -> Result<Self, String> {
        if held.exists() {
            return Err("generated fault backup already exists".into());
        }
        fs::rename(original, held).map_err(display)?;
        Ok(Self {
            original: original.to_owned(),
            held: held.to_owned(),
            active: true,
        })
    }
    fn restore(mut self) -> Result<(), String> {
        fs::rename(&self.held, &self.original).map_err(display)?;
        self.active = false;
        Ok(())
    }
}
impl Drop for Moved {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::rename(&self.held, &self.original);
        }
    }
}

#[test]
fn search_settlement_preserves_original_failure_when_terminal_audit_cannot_be_written()
-> Result<(), String> {
    let scratch = Scratch::new().map_err(display)?;
    let mut record = crate::boolean_search_record::tests::generated_record();
    let bytes = record.spec.bytes;
    let identity = record.spec.identity();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity)?;
    let (sequence, _) = journal.publish(&record.encode()?, bytes)?;
    let moved = Moved::new(
        &checkpoint_payload(&scratch.0, identity, sequence),
        &scratch.0.join("held-reservation"),
    )?;
    let why = settle_failure(
        &mut journal,
        &mut record,
        bytes,
        "original numeric-source refusal",
    );
    assert!(why.contains("original numeric-source refusal"), "{why}");
    assert!(
        why.contains("terminal audit could not be recorded"),
        "{why}"
    );
    assert!(!why.contains("refusal recorded"), "{why}");
    assert_eq!(journal.next_sequence(), sequence + 1);
    moved.restore()?;
    assert_eq!(
        journal.latest(bytes)?.ok_or("reservation lost")?.sequence,
        sequence
    );
    Ok(())
}

#[test]
fn search_settlement_retains_acknowledged_completion_and_names_late_failure() -> Result<(), String>
{
    let scratch = Scratch::new().map_err(display)?;
    // Structural generated receipt only: this test does not claim child approval.
    let mut record = crate::boolean_search_record::tests::completed_record();
    let bytes = record.spec.bytes;
    let mut journal = Journal::open(&scratch.0, NAMESPACE, record.spec.identity())?;
    journal.publish(&record.encode()?, bytes)?;
    let before = journal.latest(bytes)?.ok_or("complete absent")?;
    let why = settle_failure(
        &mut journal,
        &mut record,
        bytes,
        "original late child disappearance",
    );
    assert!(why.contains("original late child disappearance"), "{why}");
    assert!(
        why.contains("completion remains recorded; subsequent verification failed"),
        "{why}"
    );
    assert!(!why.contains("refusal recorded"), "{why}");
    same_record(&before, &journal.latest(bytes)?.ok_or("completion lost")?);
    assert_eq!(journal.next_sequence(), before.sequence + 1);
    Ok(())
}
