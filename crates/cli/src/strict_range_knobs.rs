//! Strict admission for the existing audit readers; no new policy defaults.
use std::fmt;

const NAMES: [&str; 19] = [
    "BRUTEX_CEILING",
    "BRUTEX_SCREEN_CAP",
    "BRUTEX_SCREEN_BUDGET_MS",
    "BRUTEX_TOP",
    "BRUTEX_VALIDATE",
    "BRUTEX_HORIZON_BARS",
    "BRUTEX_GRID_RUNGS",
    "BRUTEX_GRID_RESOLUTION",
    "BRUTEX_MIN_RR_BP",
    "BRUTEX_MIN_WIN_RATE_BP",
    "BRUTEX_MIN_TRADES",
    "BRUTEX_MIN_RET_OVER_DD_BP",
    "BRUTEX_MIN_WEAKEST_BP",
    "BRUTEX_MAX_MAE_PPM",
    "BRUTEX_MAX_STOP_POINTS",
    "BRUTEX_PROTECTED_EXITS",
    "BRUTEX_MIN_FILL_HEADROOM_BP",
    "BRUTEX_MIN_AVG_RR_BP",
    "BRUTEX_SIZING_RATE_BP",
];

/// Names of unusable runtime settings, without their values or filesystem paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnobRefusal {
    /// Every stated setting the actual shared audit readers cannot honor exactly.
    pub invalid: Vec<&'static str>,
}

impl fmt::Display for KnobRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "strict range runtime settings refused before computation; invalid fields: {}",
            self.invalid.join(", ")
        )
    }
}

impl std::error::Error for KnobRefusal {}

/// Whether an HTTP setting can reach the existing audit reader without fallback.
/// Names use the canonical `BRUTEX_*` spelling. Validation aliases are normalized
/// by the HTTP adapter; direct runtime values must already have the right meaning.
#[must_use]
pub fn request_value(name: &str, raw: &str) -> bool {
    if name == "BRUTEX_VALIDATE" {
        return matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "0" | "1" | "false" | "true" | "off" | "on" | "no" | "yes"
        );
    }
    value(name, raw)
}

fn value(name: &str, raw: &str) -> bool {
    use crate::knobs::{machine_count, nonnegative_floor};
    match name {
        "BRUTEX_CEILING" => machine_count(raw, crate::ceiling_limit()).is_some(),
        "BRUTEX_SCREEN_CAP" => machine_count(raw, crate::SCREEN_CAP_CEILING).is_some(),
        "BRUTEX_TOP" => nonnegative_floor(raw)
            .and_then(|count| usize::try_from(count).ok())
            .is_some_and(|count| crate::frontier::admit_top(count).is_ok()),
        "BRUTEX_VALIDATE" => matches!(raw.trim(), "0" | "1"),
        "BRUTEX_HORIZON_BARS" => {
            raw.trim().eq_ignore_ascii_case("rung") || crate::knobs::horizon_count(raw).is_some()
        }
        "BRUTEX_GRID_RUNGS" => {
            machine_count(raw, crate::rungs_within_cell_budget()).is_some_and(|count| count >= 2)
        }
        "BRUTEX_GRID_RESOLUTION" => nonnegative_floor(raw).is_some_and(|value| value > 0),
        "BRUTEX_SIZING_RATE_BP" => crate::knobs::sizing_rate(raw).is_some(),
        "BRUTEX_MIN_RR_BP"
        | "BRUTEX_MIN_WIN_RATE_BP"
        | "BRUTEX_MIN_TRADES"
        | "BRUTEX_MIN_RET_OVER_DD_BP"
        | "BRUTEX_MIN_WEAKEST_BP"
        | "BRUTEX_MAX_MAE_PPM"
        | "BRUTEX_MAX_STOP_POINTS"
        | "BRUTEX_PROTECTED_EXITS"
        | "BRUTEX_MIN_FILL_HEADROOM_BP"
        | "BRUTEX_MIN_AVG_RR_BP" => crate::knobs::policy_floor(name, raw).is_some(),
        // `BRUTEX_SCREEN_BUDGET_MS` IS IN `NAMES` AND REACHES THIS ARM ON
        // PURPOSE: it is never usable, whatever it says. The strict range audit
        // records, and a budget derives the priced cap from a wall-clock
        // calibration the identity cannot name, so any stated value is refused
        // before source admission, preparation or a slot claim. D-0685.
        _ => false,
    }
}

/// Validate the server environment plus exact request overrides before starting.
/// This does not read another active request's process-wide knob overrides.
///
/// # Errors
/// Names malformed values or values the existing readers would clamp/fall back.
pub fn validate_runtime(overrides: &[(&str, String)]) -> Result<(), KnobRefusal> {
    validate_with(overrides, |name| {
        std::env::var_os(name).map(|raw| raw.to_string_lossy().into_owned())
    })
}

/// Validate settings actually consumed by the explicit Boolean research path.
///
/// Its horizon is positional and its admission thresholds come from the explicit
/// admission policy. Of the legacy audit settings, only `BRUTEX_GRID_RUNGS`
/// changes this path's execution. Request overrides retain their usual precedence
/// over environment values; an unsupported explicit setting cannot be ignored.
///
/// # Errors
///
/// Names every unsupported legacy audit setting or invalid grid resolution before
/// policy configuration, source loading or computation.
pub fn validate_boolean_runtime() -> Result<(), KnobRefusal> {
    validate_boolean_read(crate::knobs::var)
}

fn validate_boolean_read(mut read: impl FnMut(&str) -> Option<String>) -> Result<(), KnobRefusal> {
    let invalid = NAMES
        .into_iter()
        .filter(|name| {
            read(name).is_some_and(|raw| *name != "BRUTEX_GRID_RUNGS" || !value(name, &raw))
        })
        .collect::<Vec<_>>();
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(KnobRefusal { invalid })
    }
}

#[cfg(test)]
#[path = "boolean_runtime_knob_tests.rs"]
mod boolean_tests;

fn validate_with(
    overrides: &[(&str, String)],
    mut environment: impl FnMut(&str) -> Option<String>,
) -> Result<(), KnobRefusal> {
    validate_read(|name| {
        overrides
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, raw)| raw.clone())
            .or_else(|| environment(name))
    })
}

fn validate_read(mut read: impl FnMut(&str) -> Option<String>) -> Result<(), KnobRefusal> {
    let invalid = NAMES
        .into_iter()
        .filter(|name| read(name).is_some_and(|raw| !value(name, &raw)))
        .collect::<Vec<_>>();
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(KnobRefusal { invalid })
    }
}

pub(super) fn current() -> Result<(), String> {
    validate_read(crate::knobs::var).map_err(|why| why.to_string())
}

pub(super) fn resolved() -> Result<(), String> {
    current()?;
    if crate::knobs::refused().is_some() {
        return Err("strict range runtime resolution refused a setting; no fallback result may be published".to_owned());
    }
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "assert exact refusal fields and shared scalar limits"
)]
mod tests {
    use super::*;

    #[test]
    fn strict_scalar_bounds_match_shared_readers_at_exact_limits() {
        for (name, good, bad) in [
            (
                "BRUTEX_HORIZON_BARS",
                "4294967295".to_owned(),
                "4294967296".to_owned(),
            ),
            (
                "BRUTEX_TOP",
                // The frontier reader bound, not i64::MAX: a TOP past it
                // wrote rows /frontier.json refused whole. CE-19, D-1981.
                crate::frontier::MAX_ROWS.to_string(),
                (crate::frontier::MAX_ROWS + 1).to_string(),
            ),
            (
                "BRUTEX_SCREEN_CAP",
                crate::SCREEN_CAP_CEILING.to_string(),
                (crate::SCREEN_CAP_CEILING + 1).to_string(),
            ),
            (
                "BRUTEX_GRID_RUNGS",
                crate::rungs_within_cell_budget().to_string(),
                (crate::rungs_within_cell_budget() + 1).to_string(),
            ),
            (
                "BRUTEX_CEILING",
                crate::ceiling_limit().to_string(),
                crate::ceiling_limit().saturating_add(1).to_string(),
            ),
            (
                "BRUTEX_GRID_RESOLUTION",
                i64::MAX.to_string(),
                "9223372036854775808".to_owned(),
            ),
        ] {
            assert!(value(name, &good), "{name}={good}");
            assert!(!value(name, &bad), "{name}={bad}");
            for malformed in ["", "0", "-1", "one", "2.5"] {
                assert!(!value(name, malformed), "{name}={malformed}");
            }
        }
        assert!(!value("BRUTEX_GRID_RUNGS", "1"));
        assert!(value("BRUTEX_HORIZON_BARS", " RUNG "));
        assert!(value("BRUTEX_MIN_TRADES", "0"));
        assert!(!value("BRUTEX_MIN_TRADES", "9223372036854775808"));
    }

    #[test]
    fn strict_environment_validation_refuses_without_defaulting_or_exposing_values() {
        let refusal = validate_with(&[], |name| match name {
            "BRUTEX_VALIDATE" => Some("false".to_owned()),
            "BRUTEX_MAX_STOP_POINTS" => Some("not-an-integer".to_owned()),
            "BRUTEX_SIZING_RATE_BP" => Some("5000".to_owned()),
            _ => None,
        })
        .expect_err("direct runtime spelling cannot mean a different value");
        assert_eq!(
            refusal.invalid,
            [
                "BRUTEX_VALIDATE",
                "BRUTEX_MAX_STOP_POINTS",
                "BRUTEX_SIZING_RATE_BP"
            ]
        );
        assert!(!refusal.to_string().contains("not-an-integer"));
        assert_eq!(
            validate_with(&[("BRUTEX_VALIDATE", "0".to_owned())], |name| {
                (name == "BRUTEX_VALIDATE").then(|| "false".to_owned())
            }),
            Ok(())
        );
        assert_eq!(validate_with(&[], |_| None), Ok(()));
    }

    /// The strict range audit records, so a screen budget is refused at every
    /// value while the stated screen cap stays usable. D-0685.
    #[test]
    fn strict_admission_refuses_every_screen_budget_and_keeps_the_screen_cap() {
        for raw in ["1", "5000", &u64::MAX.to_string(), "", "abc"] {
            assert!(!value("BRUTEX_SCREEN_BUDGET_MS", raw), "{raw:?}");
            assert!(!request_value("BRUTEX_SCREEN_BUDGET_MS", raw), "{raw:?}");
            let refusal = validate_with(&[("BRUTEX_SCREEN_BUDGET_MS", raw.to_owned())], |_| None)
                .expect_err("a recorded strict run cannot take a budget");
            assert_eq!(refusal.invalid, ["BRUTEX_SCREEN_BUDGET_MS"]);
            let from_environment = validate_with(&[], |name| {
                (name == "BRUTEX_SCREEN_BUDGET_MS").then(|| raw.to_owned())
            })
            .expect_err("the process environment is refused the same way");
            assert_eq!(from_environment.invalid, ["BRUTEX_SCREEN_BUDGET_MS"]);
        }
        assert!(request_value("BRUTEX_SCREEN_CAP", "7"));
        assert_eq!(
            validate_with(&[("BRUTEX_SCREEN_CAP", "7".to_owned())], |_| None),
            Ok(())
        );
    }
}
