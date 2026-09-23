#![cfg(test)]
//! Independent finite configuration-boundary checks; no market or strategy evidence.
#![allow(clippy::expect_used)]

use super::{MAX_BYTES, ResearchPolicy, SETTING, resolve_value};
use runner::admission::AdmissionFieldV1 as Field;
use std::path::Path;

const PROFILE: &str = include_str!("../../../config/intraday-research-v1.toml");

type ReadFault = fn(&Path) -> std::io::Result<()>;
thread_local! {
    static READ_FAULT: std::cell::Cell<Option<ReadFault>> = const { std::cell::Cell::new(None) };
}

pub(super) fn after_read(path: &Path) -> Result<(), String> {
    READ_FAULT
        .with(std::cell::Cell::take)
        .map_or(Ok(()), |fault| {
            fault(path).map_err(|why| format!("test could not inject policy file mutation: {why}"))
        })
}

#[test]
fn real_policy_file_mutations_inside_the_read_window_refuse_without_returning_a_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    fn append(path: &Path) -> std::io::Result<()> {
        use std::io::Write as _;
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)?
            .write_all(b"\n# changed after reading, before reauthentication\n")
    }
    fn replace(path: &Path) -> std::io::Result<()> {
        let replacement = path.with_extension("replacement.toml");
        std::fs::write(&replacement, PROFILE)?;
        std::fs::rename(replacement, path)
    }
    fn remove(path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }
    let scratch = crate::search_checkpoint::tests::Scratch::new()?;
    let path = scratch.0.join("profile.toml");
    for fault in [append as ReadFault, replace, remove] {
        std::fs::write(&path, PROFILE)?;
        READ_FAULT.with(|slot| assert!(slot.replace(Some(fault)).is_none()));
        let why = ResearchPolicy::read(&path, 5000)
            .err()
            .expect("a source mutation during read must refuse");
        assert!(
            why.contains("file changed during its bounded read")
                || why.contains("research policy source changed:"),
            "{why}"
        );
        assert!(READ_FAULT.with(std::cell::Cell::get).is_none());
    }
    std::fs::write(&path, PROFILE)?;
    assert_eq!(
        ResearchPolicy::read(&path, 5000)?.value("min_trades"),
        Some("40")
    );
    Ok(())
}

#[test]
fn exact_file_bound_accepts_complete_text_and_one_extra_byte_refuses() {
    let mut exact = PROFILE.as_bytes().to_vec();
    exact.resize(usize::try_from(MAX_BYTES).expect("fixed input bound"), b' ');
    let policy = ResearchPolicy::parse(&exact, 5000).expect("complete exact-bound profile");
    assert_eq!(policy.value("min_trades"), Some("40"));
    exact.push(b' ');
    assert!(
        ResearchPolicy::parse(&exact, 5000)
            .err()
            .expect("one extra byte must refuse")
            .contains("16384-byte")
    );
    for suffix in [b"\0".as_slice(), b"\xff", b"\nmin_trades = 200 = 1"] {
        let mut invalid = PROFILE.as_bytes().to_vec();
        invalid.extend_from_slice(suffix);
        assert!(ResearchPolicy::parse(&invalid, 5000).is_err());
    }
}

#[test]
fn loaded_profile_is_an_owned_snapshot_and_rereads_require_current_real_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let scratch = crate::search_checkpoint::tests::Scratch::new()?;
    let path = scratch.0.join("profile.toml");
    std::fs::write(&path, PROFILE)?;
    let first = ResearchPolicy::read(&path, 5000)?;
    let first_source = first.provenance().to_owned();
    let replacement = scratch.0.join("replacement.toml");
    std::fs::write(
        &replacement,
        PROFILE.replace("min_trades = 40", "min_trades = 301"),
    )?;
    std::fs::rename(&replacement, &path)?;
    let second = ResearchPolicy::read(&path, 5000)?;
    assert_eq!(first.value("min_trades"), Some("40"));
    assert_eq!(first.provenance(), first_source);
    assert_eq!(second.value("min_trades"), Some("301"));
    assert_ne!(first.provenance(), second.provenance());
    std::fs::remove_file(&path)?;
    assert!(ResearchPolicy::read(&path, 5000).is_err());
    assert_eq!(first.value("max_mae_paisa"), Some("5000"));
    assert_eq!(first.provenance(), first_source);
    assert!(
        first_source.contains("research only") && first_source.contains("BLAKE3"),
        "an owned configuration snapshot makes no forever-current file claim"
    );
    Ok(())
}

#[test]
fn signed_and_unsigned_risk_extremes_never_wrap_or_cross_domains() {
    for field in [
        Field::MinWeakestPeriodReturnPaisa,
        Field::MinPessimisticProfitPaisa,
        Field::MinOosPessimisticReturnPaisa,
    ] {
        assert_eq!(
            resolve_value(field, "\"risk*-9223372036854775808\"", 1)
                .expect("exact signed lower limit"),
            i64::MIN.to_string()
        );
        assert_eq!(
            resolve_value(field, "\"risk*0\"", u64::MAX).expect("exact zero"),
            "0"
        );
        for (raw, risk) in [
            ("\"risk*-9223372036854775808\"", 2),
            ("\"risk*9223372036854775807\"", 2),
            ("\"risk*-1\"", u64::MAX),
            ("\"risk*1\"", u64::MAX),
        ] {
            assert!(resolve_value(field, raw, risk).is_err(), "{field:?} {raw}");
        }
    }
    assert_eq!(
        resolve_value(Field::MaxMaePaisa, "\"risk*1\"", u64::MAX)
            .expect("exact unsigned upper limit"),
        u64::MAX.to_string()
    );
    for (raw, risk) in [("\"risk*2\"", u64::MAX), ("\"risk*-1\"", 1)] {
        assert!(resolve_value(Field::MaxMaePaisa, raw, risk).is_err());
    }
    assert!(resolve_value(Field::MinProfitFactorPpm, "\"risk*1\"", 1).is_err());
}

#[test]
fn edited_optional_checks_and_count_limits_explain_applied_values_without_claiming_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let scratch = crate::search_checkpoint::tests::Scratch::new()?;
    let path = scratch.0.join("optional-rejections.toml");
    let optional = PROFILE
        .replace(
            "require_white_reality_rejection = true",
            "require_white_reality_rejection = false",
        )
        .replace(
            "require_romano_wolf_rejection = true",
            "require_romano_wolf_rejection = false",
        )
        .replace(
            "max_losing_trades = 18446744073709551615",
            "max_losing_trades = 99",
        )
        .replace(
            "min_consecutive_winning_streak = 0",
            "min_consecutive_winning_streak = 2",
        );
    std::fs::write(&path, optional)?;
    let report = super::explain(&path, "50")?;
    for field in [
        Field::RequireWhiteRealityRejection,
        Field::RequireRomanoWolfRejection,
    ] {
        let row = report
            .lines()
            .find(|line| line.starts_with('|') && line.contains(field.name()))
            .ok_or("rejection requirement row missing")?;
        assert!(row.contains("Rejection not required; measured evidence still required"));
        assert!(
            row.to_ascii_lowercase().contains("when enabled"),
            "the meaning must not contradict the disabled applied limit: {row}"
        );
    }
    for (field, limit) in [
        (Field::MaxLosingTrades, "at most 99"),
        (Field::MinConsecutiveWinningStreak, "at least 2"),
    ] {
        let row = report
            .lines()
            .find(|line| line.starts_with('|') && line.contains(field.name()))
            .ok_or("edited count row missing")?;
        assert!(
            row.contains(limit),
            "the explicit edited count must be shown: {row}"
        );
    }
    assert!(report.contains("No historical strategy has passed these checks here"));
    Ok(())
}

fn request(root: &Path) -> crate::ledger_all::LedgerAllRequest<'_> {
    crate::ledger_all::LedgerAllRequest {
        vendor: "dhan",
        from: (2025, 4),
        to: (2025, 5),
        support_ppm: 1000,
        max_points: 50,
        root,
    }
}

fn isolated_case(mode: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let _serial = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set(SETTING, path.to_str().ok_or("fixture UTF-8")?);
    match mode {
        "knob-path" => assert!(ResearchPolicy::from_env(5000)?.is_none()),
        "missing-env" => assert!(ResearchPolicy::from_env(5000).is_err()),
        "precedence" => {
            let request = request(path);
            let (environment, _) = crate::ledger_all::admission_policy(&request, "policy-check")?;
            assert_eq!(environment.values().min_trades, 201);
            crate::knobs::set("BRUTEX_ADMIT_MIN_TRADES", "202");
            let (caller, active) = crate::ledger_all::admission_policy(&request, "policy-check")?;
            assert_eq!(caller.values().min_trades, 202);
            assert_eq!(caller.values().max_mae_paisa, 5000);
            assert_eq!(active.len(), 39);
            assert!(
                active
                    .iter()
                    .any(|gate| { gate.name == "min_trades" && gate.value == "202" })
            );
            let mut census = String::new();
            crate::ledger_all::render_gate_census(&mut census, &active);
            assert!(census.contains("BRUTEX_ADMIT_") && census.contains("BLAKE3"));
        }
        "empty-env" => {
            let refusal = crate::ledger_all::admission_policy(&request(path), "policy-check")
                .err()
                .ok_or("empty explicit environment silently fell back to file")?;
            assert!(refusal.contains("BRUTEX_ADMIT_MIN_TRADES="));
        }
        "invalid-file" => {
            for (line, invalid, knob) in [
                (
                    "min_support_hits = 40",
                    "min_support_hits = 0",
                    "BRUTEX_ADMIT_MIN_SUPPORT_HITS",
                ),
                (
                    "min_trades = 40",
                    "min_trades = 0",
                    "BRUTEX_ADMIT_MIN_TRADES",
                ),
                (
                    "min_win_rate_ppm = 250000",
                    "min_win_rate_ppm = 1000001",
                    "BRUTEX_ADMIT_MIN_WIN_RATE_PPM",
                ),
            ] {
                std::fs::write(path, PROFILE.replace(line, invalid))?;
                crate::knobs::set(knob, "200");
                assert!(
                    crate::ledger_all::admission_policy(&request(path), "policy-check").is_err(),
                    "a valid {knob} cannot redeem an invalid selected profile domain"
                );
                crate::knobs::clear_all();
            }
        }
        _ => return Err("unknown child mode".into()),
    }
    crate::knobs::clear_all();
    Ok(())
}

#[test]
fn isolated_process_precedence_preserves_owner_selected_path_and_refusals()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "BRUTEX_RESEARCH_POLICY_ADVERSARIAL_CHILD";
    const FIXTURE: &str = "BRUTEX_RESEARCH_POLICY_ADVERSARIAL_FIXTURE";
    if let Some(mode) = std::env::var_os(CHILD) {
        let path = std::env::var_os(FIXTURE).ok_or("fixture path missing")?;
        return isolated_case(mode.to_str().ok_or("child mode UTF-8")?, Path::new(&path));
    }
    let scratch = crate::search_checkpoint::tests::Scratch::new()?;
    let path = scratch.0.join("profile.toml");
    for mode in [
        "knob-path",
        "missing-env",
        "precedence",
        "empty-env",
        "invalid-file",
    ] {
        std::fs::write(&path, PROFILE)?;
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.args([
            "--exact",
            "research_policy::adversarial_tests::isolated_process_precedence_preserves_owner_selected_path_and_refusals",
            "--nocapture",
        ]).env(CHILD, mode).env(FIXTURE, &path).env_remove(SETTING);
        for field in Field::ALL {
            command.env_remove(format!(
                "BRUTEX_ADMIT_{}",
                field.name().to_ascii_uppercase()
            ));
        }
        match mode {
            "knob-path" => {}
            "missing-env" => {
                command.env(SETTING, scratch.0.join("missing.toml"));
            }
            "empty-env" => {
                command
                    .env(SETTING, &path)
                    .env("BRUTEX_ADMIT_MIN_TRADES", "");
            }
            _ => {
                command
                    .env(SETTING, &path)
                    .env("BRUTEX_ADMIT_MIN_TRADES", "201");
            }
        }
        let output = command.output()?;
        assert!(
            output.status.success(),
            "mode {mode}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
