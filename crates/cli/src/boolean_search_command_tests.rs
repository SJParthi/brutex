//! Public command refusals before source or output access.
use super::*;

fn arguments() -> [&'static str; 17] {
    [
        "zerodha",
        "NIFTY",
        "2025",
        "4",
        "2025",
        "5",
        "52,53",
        "5",
        "5",
        "2",
        "128",
        "1",
        "not-opened-search-fixture",
        "2025",
        "6",
        "2025",
        "8",
    ]
}

#[test]
fn optional_cli_timeframes_preserve_legacy_scope_and_canonicalize_selected_rungs()
-> Result<(), String> {
    assert_eq!(parse(&arguments())?.rungs, campaign::RungScope::ALL);
    for mask in 1..=u8::MAX {
        let scope = campaign::RungScope::from_mask(mask)?;
        let mut labels = scope.labels();
        labels.reverse();
        let selected = labels.join(",");
        let mut args = arguments().to_vec();
        args.push(&selected);
        let request = parse(&args)?;
        assert_eq!(request.rungs, scope);
        assert_eq!(request.initial, parse(&arguments())?.initial);
    }
    let mut args = arguments().to_vec();
    for invalid in ["", "1min,1min", "day", "daily", "4min"] {
        args.push(invalid);
        assert!(parse(&args).is_err());
        args.pop();
    }
    let oversized = "1".repeat(65);
    args.push(&oversized);
    assert!(parse(&args).is_err());
    assert!(!Path::new("not-opened-search-fixture").exists());
    Ok(())
}

#[test]
fn selected_launch_validation_accepts_every_canonical_subset_and_rejects_invalid_scope_before_io()
-> Result<(), String> {
    let args: Vec<String> = arguments().into_iter().map(str::to_owned).collect();
    launch::validate(&args)?;
    for mask in 1..=u8::MAX {
        let rungs = campaign::RungScope::from_mask(mask)?.labels();
        launch::validate_for_rungs(&args, &rungs)?;
        let mut reversed = rungs;
        reversed.reverse();
        launch::validate_for_rungs(&args, &reversed)?;
    }
    for invalid in [
        &[][..],
        &["1min", "1min"][..],
        &["day"][..],
        &["daily"][..],
        &["4min"][..],
    ] {
        assert!(launch::validate_for_rungs(&args, invalid).is_err());
    }
    assert!(!Path::new("not-opened-search-fixture").exists());
    Ok(())
}

#[test]
fn search_command_refuses_missing_scope_invalid_dates_and_malformed_allowances_before_io()
-> Result<(), String> {
    let base = arguments();
    for length in 0..base.len() {
        assert!(parse(base.get(..length).unwrap_or_default()).is_err());
    }
    for (index, wrong) in [
        (2, "invalid"),
        (3, "0"),
        (3, "13"),
        (4, "2024"),
        (5, "3"),
        (6, "4294967296"),
        (6, "52,,53"),
        (6, "-1"),
        (6, "999"),
        (7, "0"),
        (7, "18446744073709551616"),
        (8, "0"),
        (9, "+2"),
        (9, "02"),
        (10, "128 "),
        (10, "18446744073709551616"),
        (11, "0"),
        (11, "1.0"),
        (13, "2024"),
        (14, "5"),
        (15, "2024"),
        (16, "0"),
    ] {
        let mut args = base;
        let slot = args
            .get_mut(index)
            .ok_or("invalid declared test argument position")?;
        *slot = wrong;
        let mut out = String::new();
        assert_eq!(
            command(&args, &mut out),
            crate::MISUSED,
            "argument {index}: {wrong}"
        );
        assert!(!out.is_empty());
        assert!(
            parse(&args).is_err(),
            "refusal must precede all environment/source access"
        );
    }
    let oversized = "5".repeat(4097);
    let mut args = base;
    if let Some(bits) = args.get_mut(6) {
        *bits = &oversized;
    }
    assert!(parse(&args).is_err());
    assert!(!Path::new(base.get(12).copied().unwrap_or_default()).exists());
    Ok(())
}

#[test]
fn search_invocation_pause_allowance_does_not_change_initial_grammar_or_fixed_batch_work()
-> Result<(), String> {
    let args = arguments();
    let mut changed = args;
    if let Some(batches) = changed.get_mut(11) {
        *batches = "7";
    }
    let first = parse(&args)?;
    let next = parse(&changed)?;
    assert_eq!((first.batches, next.batches), (1, 7));
    assert_eq!(
        first.initial.initial_descriptor(),
        next.initial.initial_descriptor()
    );
    assert_eq!((first.programs, first.nodes), (next.programs, next.nodes));
    assert_eq!(
        (first.input.from, first.input.to),
        (next.input.from, next.input.to)
    );
    assert_eq!(
        (first.later_from, first.later_to),
        (next.later_from, next.later_to)
    );
    Ok(())
}
