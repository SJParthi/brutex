//! Honest sizing metadata, independent of historical source admission.
use serde_json::{Value, json};

pub(super) fn work_model() -> Value {
    match cli::boolean_search_launch::work_model() {
        Ok(model) => json!({
            "version": 1,
            "state": "lower_bound_only",
            "scope": "full_live_alphabet_per_timeframe",
            "live_conditions": model.live_conditions,
            "max_instructions": model.max_instructions,
            "conjunction_program_lower_bound": model.conjunction_program_lower_bound,
            "lower_bound_exceeds_cumulative_counter": model.lower_bound_exceeds_cumulative_counter,
            "timeframe_execution": "sequential",
            "total_programs": null,
            "eta_seconds": null,
            "refusal": null,
        }),
        Err(why) => json!({"version":1,"state":"refused","refusal":why}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizing_is_a_lower_bound_and_never_a_measured_population_or_eta() -> Result<(), String> {
        let value = work_model();
        let native = cli::boolean_search_launch::work_model()?;
        assert_eq!(value.get("state"), Some(&json!("lower_bound_only")));
        assert_eq!(
            value.get("scope"),
            Some(&json!("full_live_alphabet_per_timeframe"))
        );
        assert_eq!(
            value.get("live_conditions"),
            Some(&json!(native.live_conditions))
        );
        assert_eq!(
            value.get("conjunction_program_lower_bound"),
            Some(&json!(native.conjunction_program_lower_bound))
        );
        assert_eq!(value.get("total_programs"), Some(&Value::Null));
        assert_eq!(value.get("eta_seconds"), Some(&Value::Null));
        assert_eq!(value.get("timeframe_execution"), Some(&json!("sequential")));
        Ok(())
    }
}
