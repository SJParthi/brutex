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
        "BRUTEX_VALIDATE" => "1",
        "BRUTEX_SIZING_RATE_BP" => "5001",
        _ => "2",
    }
}

#[test]
fn boolean_rejects_each_unused_legacy_setting_without_changing_ordinary_validation() {
    for name in NAMES {
        let raw = valid_legacy_value(name);
        assert!(
            value(name, raw),
            "fixture is valid on ordinary audit path: {name}"
        );
        assert_eq!(validate_with(&[(name, raw.to_owned())], |_| None), Ok(()));
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
