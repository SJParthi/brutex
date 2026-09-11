//! Finite generated orchestration and saved-observation fault tests, not market proof.
#![expect(clippy::unwrap_used, reason = "finite generated fixture assertions")]
use super::*;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "brutex-campaign-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn state() -> State {
    let family = ResearchFamilyV1::new(crate::stored::swept_index("NIFTY").unwrap()).unwrap();
    let rows = crate::ledger_all::LEDGER_RUNGS
        .iter()
        .enumerate()
        .map(|(i, rung)| Rung {
            rung,
            status: Status::Waiting,
            reason: String::new(),
            catalogs: vec![Catalog {
                family,
                expected: brutex_core::blake3::hash(&(i as u64).to_le_bytes()),
                completion: None,
            }],
            statistics: None,
            admission: None,
        })
        .collect();
    let mut state = State {
        rungs: RungScope::ALL,
        identity: [0; 32],
        descriptor: [7; 32],
        programs: [8; 32],
        program_count: 2,
        from: (2025, 7),
        to: (2025, 8),
        horizon: 5,
        status: Status::Waiting,
        previous: None,
        rows,
    };
    state.identity = codec::identity(&state);
    state
}
#[test]
fn fixed_campaign_codec_refuses_padding_width_status_scope_and_completion_forgery() {
    let state = state();
    let raw = codec::encode(&state).unwrap();
    assert_eq!(raw.len() as u64, codec::size(1).unwrap());
    assert_eq!(codec::decode(&raw).unwrap(), state);
    for index in [0, 8, 192, 255, 256, 1160] {
        let mut changed = raw.clone();
        *changed.get_mut(index).unwrap() ^= 0x80;
        assert!(codec::decode(&changed).is_err(), "byte {index}");
    }
    assert!(codec::decode(raw.get(..raw.len() - 1).unwrap()).is_err());
    let mut complete = state.clone();
    complete.status = Status::Completed;
    assert!(codec::encode(&complete).is_err());
    let mut wrong = state.clone();
    wrong.rows.swap(0, 1);
    assert!(codec::encode(&wrong).is_err());
    let mut false_start = state;
    false_start.rows.first_mut().unwrap().status = Status::Completed;
    assert!(codec::encode(&false_start).is_err());
}
#[test]
fn exact_program_digest_binds_order_operators_and_full_fixed_wire() {
    let parse = |raw| Expression::parse(raw).unwrap();
    let programs = vec![parse("30 & 31"), parse("30 | 31"), parse("!30")];
    assert_eq!(program_digest(&programs), program_digest(&programs.clone()));
    let mut reversed = programs.clone();
    reversed.reverse();
    assert_ne!(program_digest(&programs), program_digest(&reversed));
    assert_ne!(
        program_digest(&programs),
        program_digest(&[parse("30 & 31"), parse("30 & 31"), parse("!30")])
    );
    let maximum = parse("30");
    assert_ne!(program_digest(&programs), program_digest(&[maximum]));
}

#[test]
fn every_nonempty_campaign_selection_is_isolated_and_exclusions_cannot_claim_evidence() {
    let legacy = state();
    let legacy_bytes = codec::encode(&legacy).unwrap();
    let mut identities = std::collections::HashSet::new();
    for mask in 1..=u8::MAX {
        let mut scoped = legacy.clone();
        scoped.rungs = RungScope::from_mask(mask).unwrap();
        for (index, row) in scoped.rows.iter_mut().enumerate() {
            if !scoped.rungs.contains(index) {
                row.status = Status::Excluded;
                row.catalogs.clear();
            }
        }
        scoped.identity = codec::identity(&scoped);
        assert!(identities.insert(scoped.identity));
        let raw = codec::encode(&scoped).unwrap();
        assert_eq!(codec::decode(&raw).unwrap(), scoped);
        codec::initial(&scoped).unwrap();
        if scoped.rungs == RungScope::ALL {
            assert_eq!(raw, legacy_bytes);
            assert_eq!(scoped.identity, legacy.identity);
        } else {
            assert_eq!(raw.get(..8).unwrap(), b"BRBCAM02");
            let omitted = (0..8).find(|&index| !scoped.rungs.contains(index)).unwrap();
            let mut forged = scoped.clone();
            forged.rows.get_mut(omitted).unwrap().status = Status::Completed;
            assert!(codec::encode(&forged).is_err());
            assert!(codec::same_request(&legacy, &scoped).is_err());
        }
    }
    assert_eq!(identities.len(), 255);
}
#[test]
fn checkpoint_pause_resume_pin_history_and_owner_are_independent_of_liveness() {
    let root = Temp::new();
    let mut state = state();
    let mut writer = Journal::open(&root.0, NAMESPACE, state.identity).unwrap();
    let mut position = None;
    state.status = Status::Running;
    state.rows.first_mut().unwrap().status = Status::Running;
    publish(&mut writer, &mut state, &mut position, MAX_SNAPSHOT).unwrap();
    let first = Reader::open(&root.0, state.identity, MAX_SNAPSHOT).unwrap();
    assert!(first.owner_active());
    assert_eq!(first.status(), Status::Running);
    assert!(Journal::open(&root.0, NAMESPACE, state.identity).is_err());
    state.status = Status::Paused;
    state.rows.first_mut().unwrap().status = Status::Paused;
    state.rows.first_mut().unwrap().reason = "allowance".into();
    publish(&mut writer, &mut state, &mut position, MAX_SNAPSHOT).unwrap();
    assert!(first.require_current().is_err());
    drop(writer);
    let reader = Reader::open(&root.0, state.identity, MAX_SNAPSHOT).unwrap();
    assert!(!reader.owner_active());
    assert_eq!(reader.status(), Status::Paused);
    assert!(verify_complete(&root.0, state.identity, reader.pin(), MAX_SNAPSHOT).is_err());
    let mut writer = Journal::open(&root.0, NAMESPACE, state.identity).unwrap();
    let latest = writer.latest(MAX_SNAPSHOT).unwrap().unwrap();
    verify_history(&latest, |n| writer.read(n, MAX_SNAPSHOT)).unwrap();
    state.status = Status::Running;
    state.rows.first_mut().unwrap().status = Status::Running;
    state.rows.first_mut().unwrap().reason.clear();
    publish(&mut writer, &mut state, &mut position, MAX_SNAPSHOT).unwrap();
    let previous = state.previous.unwrap();
    let path = root
        .0
        .join(NAMESPACE)
        .join(crate::identity_hex(&state.identity))
        .join(format!("{:016x}", previous.0))
        .join("payload");
    let mut bytes = fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    fs::write(path, bytes).unwrap();
    let latest = writer.latest(MAX_SNAPSHOT).unwrap().unwrap();
    assert!(verify_history(&latest, |n| writer.read(n, MAX_SNAPSHOT)).is_err());
}
fn receipt(root: &Path, namespace: &str, id: [u8; 32]) -> Link {
    let directory = root.join(namespace).join(crate::identity_hex(&id));
    fs::create_dir_all(&directory).unwrap();
    File::create(directory.join("owner.lock")).unwrap();
    let mut raw = Vec::from(*b"BRBLCM01");
    raw.extend_from_slice(&id);
    raw.extend_from_slice(&[7; 32]);
    raw.extend_from_slice(&512_u64.to_le_bytes());
    raw.extend_from_slice(&brutex_core::blake3::hash(&raw));
    let completion = brutex_core::blake3::hash(&raw);
    fs::write(directory.join("complete.bin"), raw).unwrap();
    Link {
        identity: id,
        completion,
    }
}
#[test]
fn overview_checks_exact_receipts_and_budget_without_pretending_to_check_bodies() {
    let root = Temp::new();
    let mut state = state();
    state.status = Status::Running;
    let first = state.rows.first_mut().unwrap();
    first.status = Status::Running;
    let catalog = first.catalogs.first_mut().unwrap();
    let link = receipt(&root.0, "boolean-candidates-v1", catalog.expected);
    catalog.completion = Some(link.completion);
    let mut writer = Journal::open(&root.0, NAMESPACE, state.identity).unwrap();
    let mut position = None;
    publish(&mut writer, &mut state, &mut position, MAX_SNAPSHOT).unwrap();
    drop(writer);
    let budget = codec::size(1).unwrap() + 96 + 112;
    let reader = Reader::open(&root.0, state.identity, budget).unwrap();
    assert_eq!(reader.rows().len(), 8);
    assert!(Reader::open(&root.0, state.identity, budget - 1).is_err());
    let directory = root
        .0
        .join("boolean-candidates-v1")
        .join(crate::identity_hex(&link.identity));
    assert!(
        !directory.join("body.bin").exists(),
        "overview is explicitly receipt-only"
    );
    let owner = File::open(directory.join("owner.lock")).unwrap();
    owner.lock().unwrap();
    assert!(reader.require_current().is_err());
    owner.unlock().unwrap();
    reader.require_current().unwrap();
    let path = directory.join("complete.bin");
    let raw = fs::read(&path).unwrap();
    fs::write(&path, raw.get(..111).unwrap()).unwrap();
    assert!(reader.require_current().is_err());
    fs::write(&path, &raw).unwrap();
    reader.require_current().unwrap();
    fs::remove_file(path).unwrap();
    assert!(reader.require_current().is_err());
}
#[test]
fn campaign_history_refuses_crosswired_predecessors_and_changed_acknowledged_stages() {
    let mut before = state();
    before.status = Status::Running;
    before.rows.first_mut().unwrap().status = Status::Running;
    let mut next = before.clone();
    next.rows
        .first_mut()
        .unwrap()
        .catalogs
        .first_mut()
        .unwrap()
        .completion = Some([3; 32]);
    codec::transition(&before, &next).unwrap();
    let mut lost = next.clone();
    lost.rows
        .first_mut()
        .unwrap()
        .catalogs
        .first_mut()
        .unwrap()
        .completion = None;
    assert!(codec::transition(&next, &lost).is_err());
    let mut other = next.clone();
    other.descriptor = [99; 32];
    other.identity = codec::identity(&other);
    assert!(codec::transition(&next, &other).is_err());
    assert!(codec::initial(&next).is_err());
    let receipt = CampaignReceipt {
        state: next,
        pin: [1; 32],
    };
    assert!(receipt.require_complete().is_err());
}
#[test]
fn unacknowledged_snapshot_and_exhausted_reservation_never_become_complete() {
    let root = Temp::new();
    let mut state = state();
    let mut writer = Journal::open(&root.0, NAMESPACE, state.identity).unwrap();
    assert!(Reader::open(&root.0, state.identity, MAX_SNAPSHOT).is_err());
    let directory = root
        .0
        .join(NAMESPACE)
        .join(crate::identity_hex(&state.identity));
    drop(writer);
    fs::create_dir(directory.join(format!("{MAX_CHECKPOINTS:016x}"))).unwrap();
    writer = Journal::open(&root.0, NAMESPACE, state.identity).unwrap();
    let mut position = None;
    assert!(publish(&mut writer, &mut state, &mut position, MAX_SNAPSHOT).is_err());
    assert!(position.is_none());
    assert!(bounded_reason(&"é".repeat(1000)).len() <= REASON_BYTES);
    assert!(bounded_reason(&"x".repeat(2000)).contains("truncated"));
}

pub(crate) fn coarse_fixture(fixture: &candidate::tests::Fixture) {
    use store::path::{FileKind, StorePath, Timeframe, YearMonth};
    let vendor = brutex_core::vendor::Vendor::Zerodha;
    let key = crate::stored::swept_index("NIFTY").unwrap();
    for month in [6, 7, 8] {
        let loaded =
            crate::stored::load(&fixture.root, vendor, "NIFTY", "1min", 2025, month).unwrap();
        let minutes: Vec<_> = loaded
            .bars
            .iter()
            .map(|b| store::format::Bar {
                ts_micros: b.ts_micros,
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
                open_interest: b.open_interest,
            })
            .collect();
        for seconds in [120, 180, 300, 600, 900, 1800, 3600] {
            let rows = pull::fold::fold_from_bars(
                &minutes,
                pull::fold::Bucket::of_secs(seconds).unwrap(),
                pull::fold::Bucket::MINUTE,
            )
            .unwrap();
            let path = StorePath::for_key(
                vendor,
                &key,
                Timeframe::from_secs(seconds).unwrap(),
                YearMonth::new(2025, month).unwrap(),
                FileKind::Bars,
            )
            .unwrap();
            let symbol = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
            let mut file = store::file::BarFile::open_or_create(
                &fixture.root,
                path,
                u32::from_le_bytes(symbol.get(..4).unwrap().try_into().unwrap()),
            )
            .unwrap();
            file.append(&rows).unwrap();
        }
    }
}
fn fixture_prepared(root: &Path, output: &Path, jobs: usize, out: &mut String) -> Prepared {
    let programs = vec![Expression::parse("30 & !30").unwrap()];
    let prepared = generated_prepared(
        &Request {
            rungs: RungScope::ALL,
            vendor: "zerodha",
            symbols: "NIFTY",
            from: (2025, 7),
            to: (2025, 8),
            programs: &programs,
            horizon: Horizon::bars(5).unwrap(),
            max_points: 50,
            output,
            max_rung_jobs: jobs,
        },
        out,
    )
    .unwrap();
    assert_eq!(prepared.input.source, fs::canonicalize(root).unwrap());
    prepared
}

/// Generated-only factory; production preparation retains its clean-commit gate.
pub(crate) fn generated_prepared(
    request: &Request<'_>,
    out: &mut String,
) -> Result<Prepared, String> {
    let mut input = prepared::Prepared::new(
        prepared::Input {
            vendor: request.vendor,
            symbols: request.symbols,
            from: request.from,
            to: request.to,
            horizon: request.horizon,
            max_points: request.max_points,
            output: request.output,
        },
        out,
    )?;
    input.generated_fixture()?;
    prepare_resolved(request, input, "generated-boolean-campaign-fixture")
}
#[test]
fn generated_all_eight_campaign_pauses_resumes_exactly_and_refuses_changed_source_or_child() {
    const CHILD: &str = "BRUTEX_BOOLEAN_CAMPAIGN_FIXTURE";
    if let Some(root) = std::env::var_os(CHILD) {
        let root = PathBuf::from(root);
        actual_campaign(&root);
        return;
    }
    let fixture = candidate::tests::Fixture::new().unwrap();
    fixture.prepare("NIFTY").unwrap();
    coarse_fixture(&fixture);
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args(["--exact","boolean_campaign::tests::generated_all_eight_campaign_pauses_resumes_exactly_and_refuses_changed_source_or_child","--nocapture"]);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("BRUTEX_") {
            command.env_remove(name);
        }
    }
    let result = command
        .env(CHILD, &fixture.root)
        .env("BRUTEX_STORE", &fixture.root)
        .env("BRUTEX_CHECKSUM_RECEIPTS", &fixture.root)
        .env("BRUTEX_CHECKSUM_MAX_BYTES", "67108864")
        .env("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000")
        .env(
            "BRUTEX_ADMISSION_POLICY_FILE",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/intraday-research-v1.toml"),
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
}
fn actual_campaign(root: &Path) {
    let output = root.join("campaign-output");
    fs::create_dir(&output).unwrap();
    let mut out = String::new();
    let first = fixture_prepared(root, &output, 1, &mut out);
    let id = first.identity();
    let descriptor = first.descriptor_digest();
    let budget = first.verification_bytes();
    let paused = run_prepared(first, &mut out).unwrap();
    assert!(paused.require_complete().is_err());
    let view = Reader::open(&output, id, MAX_SNAPSHOT).unwrap();
    assert_eq!(view.status(), Status::Paused);
    assert_eq!(
        view.rows()
            .iter()
            .filter(|r| r.status == Status::Completed)
            .count(),
        1
    );
    let first_link = view.rows().first().unwrap().admission.unwrap();
    let resumed = fixture_prepared(root, &output, 8, &mut out);
    assert_eq!(resumed.identity(), id);
    assert_eq!(resumed.descriptor_digest(), descriptor);
    let complete = run_prepared(resumed, &mut out).unwrap();
    complete.require_complete().unwrap();
    assert_eq!(complete.identity(), id);
    let view = Reader::open(&output, id, MAX_SNAPSHOT).unwrap();
    assert_eq!(view.rows().len(), 8);
    assert!(view.rows().iter().all(|r| r.status == Status::Completed));
    assert_eq!(view.rows().first().unwrap().admission, Some(first_link));
    verify_complete(&output, id, complete.pin(), budget).unwrap();
    let saved =
        crate::boolean_evidence::Admission::open(&output, first_link.identity, budget).unwrap();
    let mut wrong = complete.state.clone();
    wrong.programs = [4; 32];
    assert!(
        saved
            .statistics()
            .with_catalog(0, |catalog| verify_catalog_request(catalog, &wrong))
            .is_err()
    );
    wrong = complete.state.clone();
    wrong.horizon += 1;
    assert!(
        saved
            .statistics()
            .with_catalog(0, |catalog| verify_catalog_request(catalog, &wrong))
            .is_err()
    );
    drop(saved);
    refused_publication_callback(root);
    let mut repeated_out = String::new();
    let repeated = run_prepared(
        fixture_prepared(root, &output, 8, &mut out),
        &mut repeated_out,
    )
    .unwrap();
    assert_eq!(repeated.pin(), complete.pin());
    assert!(repeated_out.contains(&format!(
        "/backtest?boolean_campaign={}",
        crate::identity_hex(&id)
    )));
    assert!(repeated_out.contains("completed; snapshot"));
    assert!(repeated_out.contains(&crate::identity_hex(&complete.pin())));
    assert!(repeated_out.contains("Completed timeframes 8/8"));
    let linked = output
        .join("boolean-admission-v1")
        .join(crate::identity_hex(&first_link.identity))
        .join("body.bin");
    let bytes = fs::read(&linked).unwrap();
    fs::write(&linked, bytes.get(..bytes.len() - 1).unwrap()).unwrap();
    assert!(verify_complete(&output, id, complete.pin(), budget).is_err());
    fs::write(linked, bytes).unwrap();
    let admitted = fixture_prepared(root, &output, 8, &mut out);
    let key = crate::stored::swept_index("NIFTY").unwrap();
    let path = store::path::StorePath::for_key(
        brutex_core::vendor::Vendor::Zerodha,
        &key,
        store::path::Timeframe::DAY_1,
        store::path::YearMonth::new(2025, 6).unwrap(),
        store::path::FileKind::Bars,
    )
    .unwrap()
    .to_path_buf(root);
    let replacement = path.with_extension("replacement");
    fs::copy(&path, &replacement).unwrap();
    fs::rename(replacement, &path).unwrap();
    assert!(
        run_prepared(admitted, &mut out).is_err(),
        "retained prior daily source replacement must refuse"
    );
}

fn refused_publication_callback(root: &Path) {
    let output = root.join("refused-publication-callback");
    fs::create_dir(&output).unwrap();
    let mut out = String::new();
    let fixture = fixture_prepared(root, &output, 1, &mut out);
    let mut reached = false;
    let result = fixture
        .input
        .run("1min", &fixture.programs, &mut out, |stage| {
            if let Stage::Families(results) = stage {
                assert_eq!(results.len(), 1);
                assert!(results.first().unwrap().is_ok());
                reached = true;
                return Err("fixture refuses durable campaign stage acknowledgement".into());
            }
            Ok(())
        });
    assert!(reached);
    assert!(result.is_err());
    assert!(output.join("boolean-candidates-v1").exists());
    assert!(!output.join("boolean-statistics-v1").exists());
    assert!(!output.join("boolean-admission-v1").exists());
}
