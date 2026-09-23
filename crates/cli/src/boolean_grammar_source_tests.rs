#![cfg(test)]
//! Generated complete-calendar sources, actual execution and actual cold readers.
//! A test-only factory labels generated provenance; no clean commit is fabricated.
use super::*;
use crate::boolean_campaign::{self as campaign, tests::generated_prepared};
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;
use crate::search_checkpoint::Snapshot;
use std::fs;

const CHILD: &str = "BRUTEX_GRAMMAR_SOURCE_FIXTURE";
const TEST: &str = "boolean_grammar_campaign::source_tests::generated_grammar_executes_and_recovers_real_eight_rung_receipts";
const BYTES: u64 = 64 * 1024 * 1024;

#[test]
fn generated_grammar_executes_and_recovers_real_eight_rung_receipts() -> Result<(), String> {
    if let Some(root) = std::env::var_os(CHILD) {
        return actual(&std::path::PathBuf::from(root));
    }
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    campaign::tests::coarse_fixture(&fixture);
    let mut command = std::process::Command::new(std::env::current_exe().map_err(display)?);
    command.args(["--exact", TEST, "--nocapture"]);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("BRUTEX_") {
            command.env_remove(name);
        }
    }
    let result = command
        .env(CHILD, &fixture.root)
        .env("BRUTEX_STORE", &fixture.root)
        .env("BRUTEX_CHECKSUM_RECEIPTS", &fixture.root)
        .env("BRUTEX_CHECKSUM_MAX_BYTES", BYTES.to_string())
        .env("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000")
        .env(
            "BRUTEX_ADMISSION_POLICY_FILE",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/intraday-research-v1.toml"),
        )
        .output()
        .map_err(display)?;
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
    Ok(())
}

fn actual(root: &Path) -> Result<(), String> {
    let output = root.join("grammar-output");
    fs::create_dir(&output).map_err(display)?;
    let request = Request {
        vendor: "zerodha",
        symbols: "NIFTY",
        from: (2025, 7),
        to: (2025, 8),
        initial: Cursor::new(&[146]).map_err(|why| format!("{why:?}"))?,
        horizon: runner::outcome::Horizon::bars(5).ok_or("horizon")?,
        max_points: 50,
        programs: 1,
        nodes: 100,
        root: &output,
    };
    let allowed = Budget {
        programs: 1,
        nodes: 100,
        bytes: BYTES - (ENVELOPE + 96) as u64,
    };
    let first = Batch::prepare(request.initial.clone(), 0, 0, allowed)?;
    assert_eq!(first.programs().len(), 1);
    let expected = generated_prepared(&request.campaign(first.programs()), &mut String::new())?;
    let descriptor = expected.descriptor_digest();
    let ancestry = expected.verification_bytes();
    let pending = record_pending_batch(&request, &first, descriptor)?;
    let mut out = String::new();
    execute_with(&request, &mut out, generated_prepared)?;
    assert!(out.contains("PAUSED AT WORK ALLOWANCE"));
    let id = only_grammar_identity(&output)?;
    let first_snapshot = Snapshot::open(&output, NAMESPACE, id)?.ok_or("grammar absent")?;
    assert_eq!(first_snapshot.acknowledged, 2);
    let resumed_plan = first_snapshot.read(1, BYTES)?;
    assert_eq!(
        (resumed_plan.seal, resumed_plan.payload),
        (pending.seal, pending.payload),
        "the actual source-backed restart completes the exact reserved Plan"
    );
    let first_done = first_snapshot.read(2, BYTES)?;
    let first_link = completion(&first_done)?;
    assert_eq!(first_link.0, expected.identity());
    check_completed_retry(&request, &first, &expected, first_link, &mut out)?;

    out.clear();
    execute_with(&request, &mut out, generated_prepared)?;
    let resumed = Snapshot::open(&output, NAMESPACE, id)?.ok_or("grammar absent")?;
    assert_eq!(
        resumed.acknowledged, 4,
        "restart verifies the first batch then advances exactly once"
    );
    let unchanged = resumed.read(2, BYTES)?;
    assert_eq!(
        (unchanged.seal, unchanged.payload),
        (first_done.seal, first_done.payload)
    );
    let second = Batch::prepare(
        first.next_cursor(),
        first.work(),
        first.cumulative_programs()?,
        allowed,
    )?;
    assert_ne!(
        campaign::program_digest(first.programs()),
        campaign::program_digest(second.programs())
    );
    let second_link = completion(&resumed.read(4, BYTES)?)?;
    let second_receipt =
        campaign::verify_complete(&output, second_link.0, second_link.1, ancestry)?;
    assert_eq!(
        second_receipt.program_digest(),
        campaign::program_digest(second.programs())
    );
    check_recovery(
        &expected,
        descriptor,
        first.programs(),
        &output,
        ancestry,
        first_link,
        second_link,
    )?;
    assert!(
        complete_batch(&request, &first, [0; 32], &mut out, generated_prepared).is_err(),
        "different preflight descriptor refuses before rerunning the batch"
    );
    expected.require_current()?;
    Ok(())
}

fn record_pending_batch(
    request: &Request<'_>,
    first: &Batch,
    descriptor: [u8; 32],
) -> Result<Saved, String> {
    // This is the exact pre-pricing identity and Plan that the command records.
    // Dropping its writer models interruption before any campaign completion.
    let mut hash = Hasher::new();
    hash.update(b"brutex/boolean-grammar-campaign/v1\0");
    hash.update(&descriptor);
    hash.update(&request.initial.initial_descriptor());
    hash.update(&request.programs.to_le_bytes());
    hash.update(&request.nodes.to_le_bytes());
    let mut journal = Journal::open(request.root, NAMESPACE, hash.finalize())?;
    let raw = plan_record((0, [0; 32]), first)?;
    let (sequence, seal) = journal.publish(&raw, BYTES)?;
    assert_eq!(sequence, 1);
    let saved = journal.latest(BYTES)?.ok_or("pending Plan absent")?;
    assert_eq!((saved.sequence, saved.seal), (sequence, seal));
    assert_eq!(saved.payload, raw);
    Ok(saved)
}

fn check_completed_retry(
    request: &Request<'_>,
    first: &Batch,
    expected: &campaign::Prepared,
    first_link: ([u8; 32], [u8; 32]),
    out: &mut String,
) -> Result<(), String> {
    let output = request.root;
    let ancestry = expected.verification_bytes();
    let complete = campaign::verify_complete(output, first_link.0, first_link.1, ancestry)?;
    complete.require_complete()?;
    assert_eq!(
        complete.program_digest(),
        campaign::program_digest(first.programs())
    );
    let recorded = campaign::Reader::open(output, first_link.0, ancestry)?;
    assert_eq!(recorded.rows().len(), 8);
    assert!(recorded.rows().iter().all(|row| {
        row.status == campaign::Status::Completed
            && row
                .catalogs
                .iter()
                .all(|catalog| catalog.completion.is_some())
    }));
    let before =
        Snapshot::open(output, "boolean-campaign-v1", first_link.0)?.ok_or("campaign absent")?;
    let retry = complete_batch(
        request,
        first,
        expected.descriptor_digest(),
        out,
        generated_prepared,
    )?;
    assert_eq!(
        retry, first_link,
        "exact completed batch retry reuses its receipt"
    );
    let after =
        Snapshot::open(output, "boolean-campaign-v1", first_link.0)?.ok_or("campaign absent")?;
    assert_eq!(
        (before.latest, before.acknowledged),
        (after.latest, after.acknowledged)
    );
    let sequence = before.latest.ok_or("campaign checkpoint absent")?;
    assert_eq!(
        before.read(sequence, BYTES)?.payload,
        after.read(sequence, BYTES)?.payload
    );
    Ok(())
}

fn check_recovery(
    expected: &campaign::Prepared,
    descriptor: [u8; 32],
    programs: &[runner::expression::Expression],
    root: &Path,
    ancestry: u64,
    same: ([u8; 32], [u8; 32]),
    foreign: ([u8; 32], [u8; 32]),
) -> Result<(), String> {
    let mut reads = 0;
    recover_campaign(expected, descriptor, same.0, same.1, programs, |id, pin| {
        reads += 1;
        campaign::verify_complete(root, id, pin, ancestry)
    })?;
    assert_eq!(
        reads, 1,
        "matching typed source uses the real completion reader"
    );
    reads = 0;
    let result = recover_campaign(
        expected,
        descriptor,
        foreign.0,
        foreign.1,
        programs,
        |id, pin| {
            reads += 1;
            campaign::verify_complete(root, id, pin, ancestry)
        },
    );
    assert_eq!(
        reads, 0,
        "known foreign identity must refuse before opening its full ancestry"
    );
    assert!(result.is_err_and(|why| why.contains("freshly admitted exact batch sources")));
    let mut wrong_descriptor = descriptor;
    *wrong_descriptor.first_mut().ok_or("descriptor empty")? ^= 1;
    let result = recover_campaign(
        expected,
        wrong_descriptor,
        same.0,
        same.1,
        programs,
        |id, pin| {
            reads += 1;
            campaign::verify_complete(root, id, pin, ancestry)
        },
    );
    assert_eq!(reads, 0, "descriptor refusal must precede ancestry reads");
    assert!(result.is_err_and(|why| why.contains("freshly admitted exact batch sources")));
    Ok(())
}

fn completion(saved: &Saved) -> Result<([u8; 32], [u8; 32]), String> {
    assert_eq!(field::<8>(&saved.payload, 0)?, *DONE);
    let id = field(&saved.payload, 48)?;
    let pin = field(&saved.payload, 80)?;
    assert_ne!(
        id, [0; 32],
        "nonempty batch must record actual campaign completion"
    );
    assert_ne!(pin, [0; 32]);
    Ok((id, pin))
}

fn only_grammar_identity(root: &Path) -> Result<[u8; 32], String> {
    let entries = fs::read_dir(root.join(NAMESPACE))
        .map_err(display)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(display)?;
    assert_eq!(entries.len(), 1, "one exact grammar owner");
    let name = entries
        .first()
        .ok_or("grammar absent")?
        .file_name()
        .into_string()
        .map_err(|_| "grammar name is not text")?;
    let mut id = [0; 32];
    assert_eq!(name.len(), 64);
    for (slot, pair) in id.iter_mut().zip(name.as_bytes().chunks_exact(2)) {
        *slot =
            u8::from_str_radix(std::str::from_utf8(pair).map_err(display)?, 16).map_err(display)?;
    }
    Ok(id)
}
