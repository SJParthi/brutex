//! Units of `logs.rs` that the route tests pass by: every field value's two
//! spellings, a key refused by name, an answer over no log directory, the
//! failed-line summary, and an access line for a route that keeps none
//! (BASE-COV-API, D-7344). Every value is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use telemetry::OwnedValue;

/// **A FIELD VALUE OF EVERY KIND HAS ONE JSON AND ONE TEXT SPELLING.** A
/// number that is not one is `null` in the JSON, never the bare word no parser
/// accepts, and the page still prints it. Extremes: both infinities, NaN, the
/// widest integers, negative zero and the smallest positive float.
#[test]
fn a_field_value_of_every_kind_has_one_json_and_one_text_spelling() {
    for not_a_number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(value_json(&OwnedValue::Float(not_a_number)), "null");
        assert_eq!(
            value_text(&OwnedValue::Float(not_a_number)),
            format!("{not_a_number}")
        );
    }
    for (value, json, text) in [
        (OwnedValue::Float(1.5), "1.5", "1.5"),
        (OwnedValue::Float(-0.0), "-0", "-0"),
        (OwnedValue::Float(f64::MIN_POSITIVE), "", ""),
        (
            OwnedValue::Int(i64::MIN),
            "-9223372036854775808",
            "-9223372036854775808",
        ),
        (
            OwnedValue::Uint(u64::MAX),
            "18446744073709551615",
            "18446744073709551615",
        ),
        (OwnedValue::Bool(true), "true", "true"),
        (OwnedValue::Bool(false), "false", "false"),
        (OwnedValue::Null, "null", "null"),
        (OwnedValue::Str("a\"b".to_owned()), "\"a\\\"b\"", "a\"b"),
    ] {
        if json.is_empty() {
            let spelled: f64 = value_json(&value).parse().unwrap();
            assert_eq!(spelled, f64::MIN_POSITIVE, "{value:?}");
            let shown: f64 = value_text(&value).parse().unwrap();
            assert_eq!(shown, f64::MIN_POSITIVE, "{value:?}");
        } else {
            assert_eq!(value_json(&value), json, "{value:?}");
            assert_eq!(value_text(&value), text, "{value:?}");
        }
    }
}

/// **A KEY THAT IS TOO LONG, OR THAT HOLDS A PATH SEPARATOR, IS IGNORED BY
/// NAME.** It reaches no posting, and the answer says which it dropped.
/// Extremes: one byte past the bound, exactly the bound (kept), a forward
/// slash, a backslash, both keys bad together, and a month that is no month.
#[test]
fn a_key_too_long_or_holding_a_separator_is_ignored_by_name() {
    let long = "x".repeat(MAX_CONTRACT_ASKED + 1);
    let fits = "x".repeat(MAX_CONTRACT_ASKED);
    for name in ["contract", "code", "reason"] {
        for bad in [long.clone(), "a/b".to_owned(), "a%5Cb".to_owned()] {
            let mut ignored = Vec::new();
            let keys = keys_of(&format!("{name}={bad}"), &mut ignored);
            assert_eq!(ignored.len(), 1, "{name}={bad}: {ignored:?}");
            assert_eq!(ignored[0].0, name);
            assert!(
                keys.contract.is_none() && keys.code.is_none() && keys.reason.is_none(),
                "{name}={bad}"
            );
        }
        let mut ignored = Vec::new();
        let keys = keys_of(&format!("{name}={fits}"), &mut ignored);
        assert!(ignored.is_empty(), "{name}: {ignored:?}");
        let kept = [keys.contract, keys.code, keys.reason];
        assert_eq!(kept.iter().flatten().count(), 1, "{name}");
    }
    let mut ignored = Vec::new();
    let keys = keys_of("month=2024-13&code=a/b&contract=c/d", &mut ignored);
    assert!(keys.month.is_none() && keys.code.is_none() && keys.contract.is_none());
    let names: Vec<&str> = ignored.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names,
        ["contract", "code", "month"],
        "the order keys are read"
    );
}

/// **AN ANSWER OVER NO LOG DIRECTORY SAYS SO, ON BOTH DOORS.** No half is
/// no tail, not an empty one that reads as a quiet system.
#[test]
fn an_answer_over_no_log_directory_says_so_on_both_doors() {
    let asked = asked("limit=5");
    assert_eq!(json_across(&[], &asked, None), NO_LOG_JSON);
    let page = page_across(&[], &asked, None);
    assert!(page.contains("No log directory resolved"), "{page}");
    let tail = all_halves(&[], &asked.query);
    assert!(tail.records.is_empty() && tail.errors.is_empty());
}

/// Reports `failed_line_admitted`'s summary from the shared ration: its own
/// process, so no other test's request is held back.
const RATION_CHILD: &str = "BRUTEX_TEST_FAILED_LINE_RATION_CHILD";

/// **A WINDOW THAT HELD LINES BACK SAYS HOW MANY WHEN THE NEXT ONE OPENS.**
/// On both origin classes: past the ration a line is held, and the first
/// request of the next window is written and carries the summary, once.
#[test]
fn a_window_that_held_lines_back_says_so_when_the_next_one_opens() {
    if std::env::var_os(RATION_CHILD).is_some() {
        for (from_another_site, limit) in [
            (false, LOCAL_FAILED_LINES_PER_WINDOW),
            (true, FAILED_LINES_PER_WINDOW),
        ] {
            fresh_failed_line_window();
            let opened = FAILED_LINE_WINDOW_MS * 2;
            for _ in 0..limit {
                assert!(failed_line_admitted(from_another_site, opened));
            }
            assert!(!failed_line_admitted(from_another_site, opened));
            assert!(failed_line_admitted(
                from_another_site,
                opened + FAILED_LINE_WINDOW_MS
            ));
        }
        std::fs::write(
            std::env::var_os(RATION_CHILD).expect("named"),
            b"both classes",
        )
        .expect("the child's proof");
        return;
    }
    let proof = crate::scratch::path("failed-line-ration-proof");
    let _ = std::fs::remove_file(&proof);
    let _ = crate::isolated::rerun(
        "logs::units_tests::a_window_that_held_lines_back_says_so_when_the_next_one_opens",
        &[(RATION_CHILD, proof.as_os_str())],
    );
    assert_eq!(std::fs::read(&proof).unwrap(), b"both classes");
    let _ = std::fs::remove_file(&proof);
}

/// **A ROUTE NO `/db` PAGE READS KEEPS NO ACCESS LINE.** A refused read of a
/// route on the list writes one, the control that shows this sink would have
/// said so; the same refusal of a route off the list writes nothing.
#[test]
fn a_route_that_is_no_db_read_keeps_no_access_line() {
    let _shared = crate::emitted::sink();
    let mark = crate::emitted::mark();
    let listed = DB_READS[0];
    let now = telemetry::now_millis();
    note_access("/not-a-db-read.json", (400, 1), &[], true, now);
    note_access("/not-a-db-read.json", (500, u64::MAX), &[], true, u64::MAX);
    note_access(listed, (400, 1), &[], true, now);
    let said = crate::emitted::landed(mark, "api.access", "read refused");
    assert!(
        said.iter()
            .any(|record| text_of(record, "route") == Some(listed)),
        "the control: {said:?}"
    );
    assert!(
        !said
            .iter()
            .any(|record| text_of(record, "route") == Some("/not-a-db-read.json")),
        "{said:?}"
    );
}

/// **THE AUDIT LINE NAMES WHAT DAMAGED A STREAM, ON THE STREAM IT HIT.** A
/// span end the reopen could not read makes the paragraph loud, on the state
/// stream as on the audit one, and the words are escaped. Extremes: both
/// streams damaged at once, a last error beside them, and no audit stream.
#[test]
fn the_audit_line_names_what_damaged_a_stream_on_the_stream_it_hit() {
    assert!(streams_note(None, None).contains("keeps no audit stream"));
    let quiet = telemetry::AuditHealth::default();
    let plain = streams_note(Some(&quiet), None);
    assert!(plain.starts_with("<p class=\"lead\">"), "{plain}");
    let hurt = telemetry::AuditHealth {
        span_damage: Some("line <7>".to_owned()),
        ..telemetry::AuditHealth::default()
    };
    let state_only = streams_note(Some(&quiet), Some(&hurt));
    assert!(state_only.starts_with("<p class=\"halt\">"), "{state_only}");
    assert!(
        state_only.contains("State stream damage met on reopening: line &lt;7&gt;."),
        "{state_only}"
    );
    let both = telemetry::AuditHealth {
        last_error: Some("a failed roll".to_owned()),
        ..hurt.clone()
    };
    let loud = streams_note(Some(&both), Some(&hurt));
    assert!(
        loud.contains("Audit stream damage met on reopening"),
        "{loud}"
    );
    assert!(
        loud.contains("State stream damage met on reopening"),
        "{loud}"
    );
    assert!(
        loud.contains("Most recent failure: a failed roll."),
        "{loud}"
    );
}

/// **WHAT THE BENCH TIMES ANSWERS WHAT THE ROUTE ANSWERS.** The three doors
/// `benches/ratio/logs_dirs.rs` reads, over a store of no journal: no evidence,
/// a refused query in the route's own words, and an empty answer that is still
/// an answer.
#[test]
fn the_doors_the_bench_times_answer_over_an_empty_store() {
    let root = crate::scratch::path("logs-bench-doors");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    assert_eq!(import_evidence_over(&root, "indices", "2024-01-02"), (0, 0));
    assert_eq!(import_evidence_over(&root, "", ""), (0, 0));
    let runs = imports_json_over(&root, "feed=gdfl").expect("an answer");
    let parsed: serde_json::Value = serde_json::from_str(&runs).expect("JSON");
    assert!(parsed.is_object(), "{runs}");
    assert!(imports_json_over(&root, "feed=nosuch").is_err());
    assert!(dayflags_json_over(&root, "nosuch=1").is_err());
    let _ = std::fs::remove_dir_all(&root);
}
