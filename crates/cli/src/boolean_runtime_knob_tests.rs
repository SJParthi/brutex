#![cfg(test)]
//! Generated preflight observations; no market read or policy acceptance.
use super::*;
use crate::boolean_catalog_command::prepared::{Input, Prepared};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt as _;
use std::path::Path;
use std::process::Command;

fn valid_legacy_value(name: &str) -> &'static str {
    match name {
        // A switch: exactly `0` or `1` on every reader (P8-04, D-2723).
        "BRUTEX_VALIDATE" | "BRUTEX_PROTECTED_EXITS" => "1",
        "BRUTEX_SIZING_RATE_BP" => "5001",
        _ => "2",
    }
}

#[test]
fn boolean_rejects_each_unused_legacy_setting_without_changing_ordinary_validation() {
    for name in NAMES {
        let raw = valid_legacy_value(name);
        if name == "BRUTEX_SCREEN_BUDGET_MS" {
            // NO VALUE IS VALID ON THE ORDINARY PATH ANY MORE: the strict range
            // audit records, and a budget's cap is decided by timing. D-0685.
            assert!(!value(name, raw), "a budget cannot reach a recorded audit");
            assert_eq!(
                validate_with(&[(name, raw.to_owned())], |_| None),
                Err(KnobRefusal {
                    invalid: vec![name]
                })
            );
        } else {
            assert!(
                value(name, raw),
                "fixture is valid on ordinary audit path: {name}"
            );
            assert_eq!(validate_with(&[(name, raw.to_owned())], |_| None), Ok(()));
        }
        let observed = validate_boolean_read(|key| (key == name).then(|| raw.to_owned()));
        if name == "BRUTEX_GRID_RUNGS" {
            assert_eq!(observed, Ok(()));
        } else {
            assert_eq!(
                observed,
                Err(KnobRefusal {
                    invalid: vec![name]
                })
            );
        }
    }
    assert_eq!(validate_boolean_read(|_| None), Ok(()));
    assert_eq!(
        validate_boolean_read(|_| Some("invalid".to_owned())),
        Err(KnobRefusal {
            invalid: NAMES.to_vec()
        })
    );
}

#[test]
fn boolean_grid_resolution_keeps_the_shared_exact_boundary() -> Result<(), String> {
    let ceiling = crate::rungs_within_cell_budget();
    for count in 2..=ceiling {
        assert_eq!(
            validate_boolean_read(|name| {
                (name == "BRUTEX_GRID_RUNGS").then(|| count.to_string())
            }),
            Ok(())
        );
    }
    for raw in [
        String::new(),
        "0".to_owned(),
        "1".to_owned(),
        "-2".to_owned(),
        "2.5".to_owned(),
        "rung".to_owned(),
        ceiling
            .checked_add(1)
            .ok_or("fixture ceiling overflow")?
            .to_string(),
        u128::MAX.to_string(),
    ] {
        assert_eq!(
            validate_boolean_read(|name| { (name == "BRUTEX_GRID_RUNGS").then(|| raw.clone()) }),
            Err(KnobRefusal {
                invalid: vec!["BRUTEX_GRID_RUNGS"]
            })
        );
    }
    // The unrelated admission profile/39 fields are resolved by their own path.
    assert_eq!(
        validate_boolean_read(|name| {
            assert!(NAMES.contains(&name));
            (name == "BRUTEX_ADMIT_MIN_TRADES").then(|| "invalid".to_owned())
        }),
        Ok(())
    );
    Ok(())
}

struct ClearKnobs;
impl Drop for ClearKnobs {
    fn drop(&mut self) {
        crate::knobs::clear_all();
    }
}

fn child_case(mode: &str) -> Result<(), String> {
    let _serial = crate::knobs::serially();
    let _clear = ClearKnobs;
    crate::knobs::clear_all();
    match mode {
        "grid" | "unused" => {
            let name = if mode == "grid" {
                "BRUTEX_GRID_RUNGS"
            } else {
                "BRUTEX_HORIZON_BARS"
            };
            assert_eq!(
                validate_boolean_runtime(),
                Err(KnobRefusal {
                    invalid: vec![name]
                })
            );
        }
        "override" => {
            crate::knobs::set("BRUTEX_GRID_RUNGS", "2");
            assert_eq!(validate_boolean_runtime(), Ok(()));
            assert_eq!(
                validate_runtime(&[]),
                Err(KnobRefusal {
                    invalid: vec!["BRUTEX_GRID_RUNGS"]
                })
            );
            let policy = crate::ledger_all::exit_policy(runner::excursion::Side::Long)?;
            assert_eq!(policy.rungs().max_levels_per_axis(), 2);
        }
        "prepared" => prepared_before_configuration()?,
        _ => return Err("unknown generated child case".to_owned()),
    }
    Ok(())
}

fn prepared_before_configuration() -> Result<(), String> {
    for name in NAMES
        .into_iter()
        .filter(|name| *name != "BRUTEX_GRID_RUNGS")
    {
        crate::knobs::set(name, valid_legacy_value(name));
        let mut out = String::new();
        let result = Prepared::new(
            Input {
                vendor: "zerodha",
                symbols: "NIFTY",
                from: (2025, 4),
                to: (2025, 5),
                horizon: runner::outcome::Horizon::bars(5).ok_or("fixture horizon")?,
                max_points: 50,
                output: Path::new("/dev/null/brutex-unreachable-output"),
            },
            &mut out,
        );
        match result {
            Err(why) => assert_eq!(
                why,
                format!(
                    "Boolean catalog rejects unsupported or invalid legacy audit settings: {name}"
                )
            ),
            Ok(_) => return Err(format!("unused setting {name} authorized preparation")),
        }
        assert!(
            out.is_empty(),
            "no provenance or resolved-policy banner precedes refusal"
        );
        crate::knobs::clear_all();
    }
    Ok(())
}

#[test]
fn boolean_request_preflight_checks_override_non_utf8_and_actual_prepared_boundary()
-> Result<(), String> {
    const CHILD: &str = "BRUTEX_GENERATED_BOOLEAN_KNOB_CHILD";
    if let Ok(mode) = std::env::var(CHILD) {
        return child_case(&mode);
    }
    let executable = std::env::current_exe().map_err(|why| why.to_string())?;
    for mode in ["grid", "unused", "override", "prepared"] {
        let mut command = Command::new(&executable);
        command.env_clear().env(CHILD, mode)
            .env("BRUTEX_STORE", "/dev/null/brutex-unreachable-source")
            .args(["--exact", "audited_range_command::settings::boolean_tests::boolean_request_preflight_checks_override_non_utf8_and_actual_prepared_boundary", "--test-threads=1", "--nocapture"]);
        if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        match mode {
            "grid" | "override" => {
                command.env("BRUTEX_GRID_RUNGS", OsString::from_vec(vec![0xff]));
            }
            "unused" => {
                command.env("BRUTEX_HORIZON_BARS", OsString::from_vec(vec![0xff]));
            }
            _ => {}
        }
        let result = command.output().map_err(|why| why.to_string())?;
        assert!(
            result.status.success(),
            "generated {mode} child: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed; 0 failed"));
    }
    Ok(())
}

/// **The page `Prepared::new` writes over a stock opens with the research
/// heading that says it is gross of every charge and its corporate actions
/// are unchecked.** D-0696, AF-16.
///
/// AF-16's Boolean clause was proven on `research_heading` alone, and every
/// test that drove `Prepared::new` did it over NIFTY, so the one line that
/// puts the heading on the catalog, qualified, later-period, campaign and
/// search-launch pages could print the index's heading over a stock with
/// every test green (found by a review). `Prepared::new` reads its receipt
/// settings from the process environment, so it is driven here from a child
/// whose environment is cleared and then given a generated receipt root, its
/// two bounds, a generated store root and the admission policy file
/// `config/intraday-research-v1.toml`.
#[test]
fn the_prepared_page_over_a_stock_opens_with_its_gross_heading() -> Result<(), String> {
    const CHILD: &str = "BRUTEX_GENERATED_BOOLEAN_HEADING_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return prepared_headings();
    }
    let root = std::env::temp_dir().join(format!("brutex-boolean-heading-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|why| why.to_string())?;
    let mut command = Command::new(std::env::current_exe().map_err(|why| why.to_string())?);
    command
        .env_clear()
        .env(CHILD, "1")
        .env("BRUTEX_STORE", &root)
        .env("BRUTEX_CHECKSUM_RECEIPTS", &root)
        .env("BRUTEX_CHECKSUM_MAX_BYTES", "67108864")
        .env("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000")
        .env(
            "BRUTEX_ADMISSION_POLICY_FILE",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/intraday-research-v1.toml"),
        )
        .args([
            "--exact",
            "audited_range_command::settings::boolean_tests::the_prepared_page_over_a_stock_opens_with_its_gross_heading",
            "--test-threads=1",
            "--nocapture",
        ]);
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let result = command.output().map_err(|why| why.to_string());
    let _ = std::fs::remove_dir_all(&root);
    let result = result?;
    assert!(
        result.status.success(),
        "generated heading child: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed; 0 failed"));
    Ok(())
}

/// The child of `the_prepared_page_over_a_stock_opens_with_its_gross_heading`.
fn prepared_headings() -> Result<(), String> {
    let _serial = crate::knobs::serially();
    let _clear = ClearKnobs;
    crate::knobs::clear_all();
    let root = crate::store_root()?;
    let stock = format!(
        "{}{}",
        // The prepared catalogue is a page over many instrument-months, so it
        // opens with the pooled banner and not the single-run one. D-1705.
        crate::STORED_POOLED_PROVENANCE,
        runner::audit::CostScope::CashEquity.report_note()
    );
    for (symbols, cash) in [
        ("RELIANCE", true),
        ("NIFTY,RELIANCE", true),
        ("NIFTY", false),
    ] {
        let mut out = String::new();
        let prepared = Prepared::new(
            Input {
                vendor: "zerodha",
                symbols,
                from: (2025, 4),
                to: (2025, 5),
                horizon: runner::outcome::Horizon::bars(5).ok_or("fixture horizon")?,
                max_points: 50,
                output: &root,
            },
            &mut out,
        )
        .map_err(|why| format!("{symbols}: {why}"))?;
        let heading = crate::boolean_catalog_command::prepared::research_heading(&prepared.scope);
        assert!(
            out.starts_with(&heading),
            "{symbols}: the page opens with the research heading:\n{out}"
        );
        assert_eq!(
            out.starts_with(&stock),
            cash,
            "{symbols}: a stock's page opens with the gross and corporate-action note, \
             and an index's does not:\n{out}"
        );
        assert_eq!(
            out.contains(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
            cash,
            "{symbols}:\n{out}"
        );
    }
    Ok(())
}
