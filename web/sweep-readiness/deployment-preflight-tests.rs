use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-stage-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("unique test directory");
        Self(path)
    }
    fn dir(&self, path: &str) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(&path).expect("fixture directory");
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("fixture cleanup");
    }
}

fn digest(raw: &[u8]) -> String {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(raw);
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
fn proof() -> Vec<u8> {
    let checks: Vec<_> = REQUIRED_CHECKS
        .iter()
        .map(|name| json!({"name":name,"passed":true,"exit":0}))
        .collect();
    format!(
        "window.BRUTEX_AUDIT_CHECKS = {};\n",
        json!({"complete":true,"stable":true,"checks":checks})
    )
    .into_bytes()
}
fn artifact(role: Role, path: PathBuf, raw: &[u8]) -> Artifact {
    fs::write(&path, raw).expect("fixture artifact");
    Artifact {
        role,
        path,
        bytes: raw.len().to_string(),
        blake3: digest(raw),
    }
}
fn plan(scratch: &Scratch) -> Plan {
    let web = scratch.dir("web");
    scratch.dir("web/build/_app");
    let store = scratch.dir("store");
    let artifacts = vec![
        artifact(
            Role::Api,
            scratch.0.join("api"),
            b"pinned test executable bytes",
        ),
        artifact(
            Role::Cli,
            scratch.0.join("cli"),
            b"different pinned test bytes",
        ),
        artifact(Role::Checks, scratch.0.join("checks.js"), &proof()),
        artifact(
            Role::Web,
            web.join("build/index.html"),
            b"<html>fixture</html>",
        ),
        artifact(
            Role::Web,
            web.join("build/backtest.html"),
            b"<html>backtest fixture</html>",
        ),
        artifact(
            Role::Web,
            web.join("build/_app/version.json"),
            b"{\"version\":\"fixture\"}",
        ),
    ];
    Plan {
        schema_version: 1,
        source_commit: "a".repeat(40),
        artifact_bytes: "1048576".into(),
        observation_bytes: "1610612736".into(),
        replay_nodes: "1000000".into(),
        required_evidence_bytes: "161371458".into(),
        web_root: web,
        source_store: store.clone(),
        output_root: store.clone(),
        server_store: store,
        masters_root: scratch.dir("masters"),
        logs_root: scratch.dir("logs"),
        address: "127.0.0.1:8080".parse().expect("loopback"),
        artifacts,
        mandatory_gates: None,
    }
}

#[test]
fn offline_and_missing_mandatory_admission_never_invoke_live_observation() {
    let scratch = Scratch::new();
    let plan = plan(&scratch);
    for mode in [
        Mode {
            offline: true,
            require_release_gates: false,
        },
        Mode {
            offline: false,
            require_release_gates: true,
        },
        Mode {
            offline: true,
            require_release_gates: true,
        },
    ] {
        let result = preflight(&plan, mode, |_, _| panic!("network callback forbidden")).unwrap();
        let result = &result.body;
        assert_eq!(result["stage"]["state"], "pinned_artifacts_verified");
        assert_eq!(result["mandatory_gates"]["state"], "blocked");
        assert_eq!(result["activation_admission"], "blocked_by_mandatory_gates");
        assert_eq!(result["live_status_checked"], false);
        assert_eq!(result["observations"], json!([]));
        assert_eq!(result["activation"], "not_authorized_by_this_probe");
    }
}

#[test]
fn explicit_modes_refuse_ambiguous_arguments() {
    let parse = |args: &[&str]| arguments(args.iter().map(std::ffi::OsString::from));
    assert_eq!(
        parse(&["--offline", "--require-release-gates", "plan"])
            .unwrap()
            .0,
        Mode {
            offline: true,
            require_release_gates: true
        }
    );
    assert!(!parse(&["plan"]).unwrap().0.offline);
    for args in [
        vec![],
        vec!["--offline"],
        vec!["--offline", "--offline", "plan"],
        vec!["--typo", "plan"],
        vec!["plan", "extra"],
        vec!["plan", "--offline"],
    ] {
        assert!(parse(&args).is_err());
    }
}

#[test]
fn exact_artifacts_full_checks_and_matching_output_root_pass_only_staging() {
    let scratch = Scratch::new();
    let plan = plan(&scratch);
    let result = verify_stage(&plan).expect("bounded reviewed fixture");
    let result = &result.body;
    assert_eq!(result["state"], "pinned_artifacts_verified");
    assert_eq!(result["artifact_count"], "6");
    assert_eq!(result["observation_byte_limit"], "1610612736");
    assert!(result.get("activation").is_none());
}

#[test]
fn changed_missing_extra_alias_and_symlink_artifacts_refuse() {
    let scratch = Scratch::new();
    let mut plan = plan(&scratch);
    let target = plan.artifacts[0].path.clone();
    fs::write(&target, b"changed").unwrap();
    assert!(verify_stage(&plan).unwrap_err().contains("length"));
    plan.artifacts[0] = artifact(Role::Api, target.clone(), b"restored fixture");
    fs::write(plan.web_root.join("build/unpinned.js"), b"extra").unwrap();
    assert!(verify_stage(&plan).unwrap_err().contains("unpinned"));
    fs::remove_file(plan.web_root.join("build/unpinned.js")).unwrap();
    let real = scratch.0.join("real-api");
    fs::rename(&target, &real).unwrap();
    std::os::unix::fs::symlink(&real, &target).unwrap();
    assert!(verify_stage(&plan).is_err());
    fs::remove_file(&target).unwrap();
    assert!(verify_stage(&plan).is_err());
}

#[test]
fn held_verified_file_detects_later_replacement_and_mutation() {
    let scratch = Scratch::new();
    let path = scratch.0.join("artifact");
    fs::write(&path, b"original").unwrap();
    let held = files::verify(&path, 8, &digest(b"original")).unwrap();
    let replacement = scratch.0.join("replacement");
    fs::write(&replacement, b"original").unwrap();
    fs::rename(replacement, &path).unwrap();
    assert!(held.require_current().is_err());
    let held = files::verify(&path, 8, &digest(b"original")).unwrap();
    fs::write(&path, b"modified").unwrap();
    assert!(held.require_current().is_err());
}

#[test]
fn incomplete_unstable_wrong_order_and_focused_checks_refuse() {
    let raw = proof();
    validate_checks(&raw).unwrap();
    let text = String::from_utf8(raw).unwrap();
    for (old, new) in [
        ("\"stable\":true", "\"stable\":false"),
        ("\"complete\":true", "\"complete\":false"),
        ("\"passed\":true", "\"passed\":false"),
        ("\"exit\":0", "\"exit\":1"),
        ("Workspace integration", "Sweep tests"),
    ] {
        assert!(
            validate_checks(text.replacen(old, new, 1).as_bytes()).is_err(),
            "{old}"
        );
    }
    assert!(
        validate_checks(
            b"window.BRUTEX_AUDIT_CHECKS = {\"complete\":true,\"stable\":true,\"checks\":[]};"
        )
        .is_err()
    );
}

#[test]
fn mismatched_evidence_root_insufficient_budget_and_nonloopback_refuse() {
    let scratch = Scratch::new();
    let mut plan = plan(&scratch);
    plan.server_store = scratch.dir("wrong-store");
    assert!(verify_stage(&plan).unwrap_err().contains("output root"));
    plan.server_store = plan.output_root.clone();
    plan.observation_bytes = "67108864".into();
    assert!(
        verify_stage(&plan)
            .unwrap_err()
            .contains("recorded complete evidence")
    );
    plan.observation_bytes = "1610612736".into();
    plan.address = "0.0.0.0:8080".parse().unwrap();
    assert!(verify_stage(&plan).unwrap_err().contains("loopback"));
}

#[test]
fn strict_manifest_refuses_duplicate_keys_unknown_fields_and_numeric_rounding() {
    assert!(
        serde_json::from_str::<Artifact>(
            r#"{"role":"api","path":"/tmp/a","bytes":"1","bytes":"2","blake3":"a"}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<Artifact>(
            r#"{"role":"api","path":"/tmp/a","bytes":9007199254740993,"blake3":"a"}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<Artifact>(
            r#"{"role":"api","path":"/tmp/a","bytes":"1","blake3":"a","ignore_failure":true}"#
        )
        .is_err()
    );
}

#[test]
fn raw_report_json_preserves_exact_numbers_and_rejects_recursive_duplicate_keys() {
    let raw = br#"{"large":9007199254740993,"maximum":18446744073709551615,"minimum":-9223372036854775808,"duration":0.125,"zero":0,"items":[null,true,"exact",{"same":1},{"same":2}],"escaped":"\u0061"}"#;
    let decoded = files::json(raw, PLAN_BYTES).unwrap();
    assert_eq!(decoded, serde_json::from_slice::<Value>(raw).unwrap());
    assert_eq!(decoded["large"].as_u64(), Some(9_007_199_254_740_993));
    assert_eq!(decoded["maximum"].as_u64(), Some(u64::MAX));
    assert_eq!(decoded["minimum"].as_i64(), Some(i64::MIN));
    assert_eq!(decoded["duration"].as_f64(), Some(0.125));
    assert_eq!(decoded["escaped"], "a");
    for raw in [
        r#"{"same":1,"same":2}"#,
        r#"{"items":[{"same":1,"same":2}]}"#,
        r#"{"outer":{"same":1,"\u0073ame":2}}"#,
        r#"{"extra":{"ignored":null,"ignored":null}}"#,
    ] {
        assert!(
            files::json(raw.as_bytes(), PLAN_BYTES)
                .unwrap_err()
                .contains("duplicated")
        );
    }
}

#[test]
fn strict_report_json_keeps_byte_recursion_and_complete_input_bounds() {
    assert_eq!(files::json(b"{}", 2).unwrap(), json!({}));
    assert_eq!(
        files::json(b"{}", 1).unwrap_err(),
        "JSON input exceeds its byte limit"
    );
    let nested = format!("{}null{}", "[".repeat(32), "]".repeat(32));
    assert!(files::json(nested.as_bytes(), PLAN_BYTES).is_ok());
    let too_deep = format!("{}null{}", "[".repeat(256), "]".repeat(256));
    assert!(
        files::json(too_deep.as_bytes(), PLAN_BYTES)
            .unwrap_err()
            .contains("deeply nested")
    );
    for raw in ["{} {}", "{", "[0,]", "{\"x\":1e999}", "/*comment*/{}"] {
        assert!(files::json(raw.as_bytes(), PLAN_BYTES).is_err(), "{raw}");
    }
}

#[test]
fn duplicate_checks_and_status_fields_are_refused_without_observation() {
    let raw = String::from_utf8(proof()).unwrap();
    for (old, replacement) in [
        (r#""complete":true"#, r#""complete":false,"complete":true"#),
        (r#""passed":true"#, r#""passed":false,"pa\u0073sed":true"#),
    ] {
        let duplicate = raw.replacen(old, replacement, 1);
        assert_ne!(duplicate, raw);
        assert!(
            validate_checks(duplicate.as_bytes())
                .unwrap_err()
                .contains("duplicated")
        );
    }
    for (route, body) in [
        ("/pull/run.json", r#"{"running":true,"running":false}"#),
        (
            "/backtest/run.json",
            r#"{"running":{"in_flight":true,"in_flight":false,"where":"cli"}}"#,
        ),
    ] {
        let wire = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let parsed = response_json(wire.as_bytes());
        assert!(parsed.is_err());
        assert_eq!(observation(route, parsed)["state"], "unknown");
    }
}

#[test]
fn complete_http_body_required_and_body_text_is_not_exposed() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
    assert_eq!(response_json(raw).unwrap(), json!({}));
    for raw in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n{}".as_slice(),
        b"HTTP/1.1 503 Refused\r\nContent-Length: 2\r\n\r\n{}",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{}",
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
    ] {
        assert!(response_json(raw).is_err());
    }
    let out = observation(
        "/pull/recovery.json",
        Ok(json!({"private_runtime_details":"must never be projected"})),
    );
    assert_eq!(out["state"], "unknown");
    assert!(!out.to_string().contains("private_runtime_details"));
}

#[test]
fn active_unknown_paused_and_terminal_observations_never_authorize_handoff() {
    for (route, input, state) in [
        ("/pull/run.json", json!({"running":true}), "active"),
        ("/pull/run.json", json!({}), "unknown"),
        (
            "/autopilot.json",
            json!({"state":"paused","pull_active":false}),
            "paused",
        ),
        (
            "/autopilot.json",
            json!({"state":"paused","pull_active":true}),
            "active",
        ),
        ("/backtest/run.json", json!({"running":null}), "unknown"),
        (
            "/backtest/run.json",
            json!({"running":{"where":"cli","in_flight":false,"status":"unknown"}}),
            "unknown",
        ),
        (
            "/backtest/run.json",
            json!({"running":{"where":"browser","in_flight":false}}),
            "not_observed_active",
        ),
    ] {
        let out = observation(route, Ok(input));
        assert_eq!(out["state"], state);
        assert!(out.get("activation").is_none());
    }
}

fn pin(path: PathBuf, raw: &[u8]) -> gates::Pin {
    fs::write(&path, raw).unwrap();
    gates::Pin {
        path,
        bytes: raw.len().to_string(),
        blake3: digest(raw),
    }
}

fn pinned_json(path: PathBuf, value: &Value) -> gates::Pin {
    pin(path, value.to_string().as_bytes())
}

fn pin_value(pin: &gates::Pin) -> Value {
    json!({"path":pin.path,"bytes":pin.bytes,"blake3":pin.blake3})
}

fn gated_plan(scratch: &Scratch) -> Plan {
    let mut plan = plan(scratch);
    let source = fs::canonicalize(scratch.dir("source")).unwrap();
    scratch.dir("source/crates/widget/src");
    let mut sources = Vec::new();
    for (path, raw) in [
        ("Cargo.toml", "[workspace]\nmembers=['crates/widget']\n"),
        ("Cargo.lock", "# generated fixture lock\n"),
        ("crates/widget/Cargo.toml", "[package]\nname='widget'\n"),
        (
            "crates/widget/src/lib.rs",
            "pub fn generated(x: bool) -> u8 { if x { 1 } else { 2 } }\n",
        ),
    ] {
        fs::write(source.join(path), raw).unwrap();
        sources.push(gates::Source {
            path: path.into(),
            bytes: raw.len().to_string(),
            blake3: digest(raw.as_bytes()),
        });
    }
    let coverage = pinned_json(
        scratch.0.join("coverage.json"),
        &json!({
            "type":"llvm.coverage.json.export","data":[{"files":[{
                "filename":source.join("crates/widget/src/lib.rs"),
                "summary":{"lines":{"count":3,"covered":3,"percent":100},"branches":{"count":2,"covered":2,"percent":100}}
            }]}]
        }),
    );
    let mutant = json!({"file":"crates/widget/src/lib.rs","package":"widget", "function":null,
        "span":{"start":{"line":1,"column":1},"end":{"line":1,"column":2}},
        "replacement":"false","genre":"BinaryOperator","name":"generated exact mutant"});
    let census = pinned_json(scratch.0.join("mutants.json"), &json!([mutant.clone()]));
    let outcomes = pinned_json(
        scratch.0.join("outcomes.json"),
        &json!({"outcomes":[
            {"scenario":"Baseline","summary":"Success","phase_results":[
                {"phase":"Build","process_status":"Success"},{"phase":"Test","process_status":"Success"}]},
            {"scenario":{"Mutant":mutant},"summary":"CaughtMutant","phase_results":[
                {"phase":"Build","process_status":"Success"},{"phase":"Test","process_status":{"Failure":101}}]}
        ]}),
    );
    let mut required = gates::Requirements {
        source_root: source,
        crates: vec!["widget".into()],
        modules: vec!["crates/widget/src/lib.rs".into()],
        sources,
        evidence: pin(scratch.0.join("mandatory.json"), b"temporary fixture"),
    };
    let evidence = json!({"schema_version":1,"source_commit":plan.source_commit,
        "scope_digest":gates::scope_digest(&plan.source_commit, &required),"complete":true,
        "coverage":[{"crate_name":"widget","full_crate":true,"branch_instrumented":true,"report":pin_value(&coverage)}],
        "mutations":[{"module":"crates/widget/src/lib.rs","full_module":true,"census":pin_value(&census),"outcomes":pin_value(&outcomes),"compiler_failures":[]}]
    });
    required.evidence = pinned_json(required.evidence.path, &evidence);
    plan.schema_version = 2;
    plan.mandatory_gates = Some(required);
    plan
}

fn edit_evidence(plan: &mut Plan, edit: impl FnOnce(&mut Value)) {
    let required = plan.mandatory_gates.as_mut().unwrap();
    let mut value = serde_json::from_slice(&fs::read(&required.evidence.path).unwrap()).unwrap();
    edit(&mut value);
    required.evidence = pinned_json(required.evidence.path.clone(), &value);
}

fn edit_report(plan: &mut Plan, kind: &str, field: &str, edit: impl FnOnce(&mut Value)) {
    edit_evidence(plan, |evidence| {
        let report = &mut evidence[kind][0][field];
        let path = PathBuf::from(report["path"].as_str().unwrap());
        let mut value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut value);
        *report = pin_value(&pinned_json(path, &value));
    });
}

#[test]
fn duplicate_coverage_census_and_outcome_fields_cannot_replace_failures() {
    for (kind, field, old, replacement) in [
        ("coverage", "report", r#""data":"#, r#""data":[],"data":"#),
        (
            "coverage",
            "report",
            r#""covered":3"#,
            r#""covered":0,"\u0063overed":3"#,
        ),
        (
            "mutations",
            "census",
            r#""replacement":"false""#,
            r#""replacement":"true","replacement":"false""#,
        ),
        (
            "mutations",
            "outcomes",
            r#""summary":"CaughtMutant""#,
            r#""summary":"MissedMutant","summary":"CaughtMutant""#,
        ),
        (
            "mutations",
            "outcomes",
            r#""process_status":{"Failure":101}"#,
            r#""process_status":"Success","process_status":{"Failure":101}"#,
        ),
    ] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        gate(&plan).unwrap();
        edit_evidence(&mut plan, |evidence| {
            let report = &mut evidence[kind][0][field];
            let path = PathBuf::from(report["path"].as_str().unwrap());
            let raw = fs::read_to_string(&path).unwrap();
            let duplicate = raw.replacen(old, replacement, 1);
            assert_ne!(duplicate, raw, "the targeted raw report field exists");
            *report = pin_value(&pin(path, duplicate.as_bytes()));
        });
        assert!(
            gate(&plan).unwrap_err().contains("duplicated"),
            "{kind}/{field}"
        );
    }
}

fn gate(plan: &Plan) -> Result<(), String> {
    gates::verify(&plan.source_commit, plan.mandatory_gates.as_ref()).map(drop)
}

#[test]
fn exact_source_full_crate_and_complete_mutant_census_pass_only_mandatory_gates() {
    let scratch = Scratch::new();
    let plan = gated_plan(&scratch);
    let result = preflight(
        &plan,
        Mode {
            offline: true,
            require_release_gates: true,
        },
        |_, _| panic!("offline"),
    )
    .unwrap();
    let result = &result.body;
    assert_eq!(result["mandatory_gates"]["state"], "passed");
    assert_eq!(
        result["activation_admission"],
        "mandatory_gates_passed_handoff_still_required"
    );
    assert_eq!(result["activation"], "not_authorized_by_this_probe");
    assert_eq!(result["live_status_checked"], false);
}

#[test]
fn terminal_observation_rechecks_artifact_source_and_measurement_bytes() {
    for changed in [
        "api",
        "source/crates/widget/src/lib.rs",
        "coverage.json",
        "outcomes.json",
        "mandatory.json",
    ] {
        let scratch = Scratch::new();
        let plan = gated_plan(&scratch);
        let mut calls = 0;
        let result = preflight(
            &plan,
            Mode {
                offline: false,
                require_release_gates: true,
            },
            |_, _| {
                calls += 1;
                if calls == 1 {
                    fs::write(
                        scratch.0.join(changed),
                        b"changed after successful cold admission",
                    )
                    .unwrap();
                }
                Err("generated status is unavailable; no network operation".into())
            },
        );
        assert_eq!(
            calls, 4,
            "cold admission passed before the generated late edit"
        );
        assert!(
            result.unwrap_err().contains("changed during preflight"),
            "{changed}"
        );
    }
}

#[test]
fn terminal_observation_rechecks_complete_source_and_dashboard_censuses() {
    for added in ["source/crates/widget/src/new.rs", "web/build/new.html"] {
        let scratch = Scratch::new();
        let plan = gated_plan(&scratch);
        let mut calls = 0;
        let result = preflight(
            &plan,
            Mode {
                offline: false,
                require_release_gates: true,
            },
            |_, _| {
                calls += 1;
                if calls == 1 {
                    fs::write(scratch.0.join(added), b"new unmeasured file").unwrap();
                }
                Err("generated status is unavailable; no network operation".into())
            },
        );
        assert_eq!(calls, 4, "the earlier census really passed");
        assert_eq!(
            result.unwrap_err(),
            "previously verified complete file census changed during preflight",
            "{added}"
        );
    }
}

#[test]
fn settled_report_retains_original_guards_without_authorizing_activation() {
    let scratch = Scratch::new();
    let plan = gated_plan(&scratch);
    let mut calls = 0;
    let report = preflight(
        &plan,
        Mode {
            offline: false,
            require_release_gates: true,
        },
        |_, _| {
            calls += 1;
            Err("generated status unavailable".into())
        },
    )
    .unwrap();
    assert_eq!(calls, 4);
    assert_eq!(report.body["mandatory_gates"]["state"], "passed");
    assert_eq!(report.body["activation"], "not_authorized_by_this_probe");
    report.retained.require_current().unwrap();
    fs::write(scratch.0.join("api"), b"later mutation").unwrap();
    assert!(report.retained.require_current().is_err());
}

#[test]
fn retained_census_rescan_keeps_its_original_entry_bound() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("one.rs"), b"fn one() {}\n").unwrap();
    let retained = files::Retained {
        files: Vec::new(),
        inventories: vec![files::Inventory::capture(&scratch.0, 1, true).unwrap()],
    };
    retained.require_current().unwrap();
    fs::write(scratch.0.join("two.rs"), b"fn two() {}\n").unwrap();
    assert_eq!(
        retained.require_current().unwrap_err(),
        "bundle entry limit exceeded"
    );
}

#[test]
fn absent_stale_incomplete_foreign_or_missing_mandatory_measurements_refuse() {
    assert!(
        gates::verify(&"a".repeat(40), None)
            .unwrap_err()
            .contains("missing")
    );
    for (field, replacement) in [
        ("source_commit", json!("b".repeat(40))),
        ("scope_digest", json!("0".repeat(64))),
        ("complete", json!(false)),
        ("coverage", json!([])),
        ("mutations", json!([])),
    ] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_evidence(&mut plan, |value| {
            value[field] = replacement;
        });
        assert!(gate(&plan).is_err(), "{field}");
    }
}

#[test]
fn coverage_requires_exact_line_branch_counts_and_all_source_files() {
    for (kind, covered) in [("lines", 2), ("branches", 1)] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_report(&mut plan, "coverage", "report", |report| {
            report["data"][0]["files"][0]["summary"][kind]["covered"] = json!(covered);
            // An unchanged rounded 100 is deliberately not accepted.
        });
        assert!(gate(&plan).unwrap_err().contains("below 100"));
    }
    for number in ["3.0", "3e0", "18446744073709551616", "-1"] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_evidence(&mut plan, |evidence| {
            let report = &mut evidence["coverage"][0]["report"];
            let path = PathBuf::from(report["path"].as_str().unwrap());
            let raw = fs::read_to_string(&path).unwrap();
            let changed = raw
                .replace("\"count\":3", &format!("\"count\":{number}"))
                .replace("\"covered\":3", &format!("\"covered\":{number}"));
            assert_ne!(changed, raw);
            *report = pin_value(&pin(path, changed.as_bytes()));
        });
        assert!(gate(&plan).unwrap_err().contains("below 100"), "{number}");
    }
    for field in ["full_crate", "branch_instrumented"] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_evidence(&mut plan, |value| {
            value["coverage"][0][field] = json!(false);
        });
        assert!(gate(&plan).is_err());
    }
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    edit_report(&mut plan, "coverage", "report", |report| {
        report["data"][0]["files"] = json!([]);
    });
    assert!(gate(&plan).unwrap_err().contains("omits source"));
}

#[test]
fn survivors_timeouts_unviable_and_incomplete_mutant_census_never_clear_release() {
    for status in ["MissedMutant", "Timeout", "Success"] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_report(&mut plan, "mutations", "outcomes", |report| {
            report["outcomes"][1]["summary"] = json!(status);
        });
        assert!(gate(&plan).unwrap_err().contains("unresolved"), "{status}");
    }
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    edit_report(&mut plan, "mutations", "outcomes", |report| {
        report["outcomes"].as_array_mut().unwrap().pop();
    });
    assert!(
        gate(&plan)
            .unwrap_err()
            .contains("exact full-module census")
    );
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    edit_evidence(&mut plan, |value| {
        value["mutations"][0]["full_module"] = json!(false);
    });
    assert!(gate(&plan).unwrap_err().contains("partial"));
}

#[test]
fn genuine_cargo_test_phase_shape_is_retained_by_complete_census() {
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    // Same raw phase/status shape as the retained grammar and search-allocation
    // cargo-mutants outcomes; paths/durations here are generated fixture metadata.
    edit_report(&mut plan, "mutations", "outcomes", |report| {
        report["outcomes"][1]["phase_results"] = json!([
            {"phase":"Build","duration":15.606030041,"process_status":"Success",
                "argv":["/native/cargo","test","--no-run","--verbose","--package=widget@0.1.0"]},
            {"phase":"Test","duration":3.040973917,"process_status":{"Failure":101},
                "argv":["/native/cargo","test","--verbose","--package=widget@0.1.0","--lib","--","--test-threads=2"]}
        ]);
    });
    gate(&plan).unwrap();
}

#[test]
fn caught_summary_requires_both_completed_phases_in_order() {
    let build = json!({"phase":"Build","process_status":"Success"});
    let test = json!({"phase":"Test","process_status":{"Failure":101}});
    for phases in [
        Value::Null,
        json!({"Build":"Success","Test":{"Failure":101}}),
        json!([]),
        json!([build]),
        json!([test]),
        json!([test, build]),
        json!([build, test, test]),
        json!([{}, test]),
        json!([build, {}]),
    ] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_report(&mut plan, "mutations", "outcomes", |report| {
            report["outcomes"][1]["phase_results"] = phases;
        });
        assert!(gate(&plan).unwrap_err().contains("caught mutation"));
    }
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    edit_report(&mut plan, "mutations", "outcomes", |report| {
        report["outcomes"][1]
            .as_object_mut()
            .unwrap()
            .remove("phase_results");
    });
    assert_eq!(
        gate(&plan).unwrap_err(),
        "caught mutation phase evidence is missing"
    );
}

#[test]
fn caught_summary_never_overrides_success_timeout_or_infrastructure_status() {
    for (phase, status) in [
        (0, json!("Timeout")),
        (0, json!({"Failure":101})),
        (1, json!("Success")),
        (1, json!("Timeout")),
        (1, json!({"Signalled":9})),
        (1, json!({"Error":"process spawn failed"})),
        (1, json!({"Failure":0})),
        (1, json!({"Failure":1})),
        (1, json!({"Failure":100})),
        (1, json!({"Failure":137})),
        (1, json!({"Failure":101,"Timeout":true})),
    ] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_report(&mut plan, "mutations", "outcomes", |report| {
            report["outcomes"][1]["phase_results"][phase]["process_status"] = status;
        });
        assert!(
            gate(&plan)
                .unwrap_err()
                .contains("completed successful Build")
        );
    }
}

fn make_unviable(plan: &mut Plan, scratch: &Scratch, log: &str) {
    edit_report(plan, "mutations", "outcomes", |report| {
        report["outcomes"][1]["summary"] = json!("Unviable");
        report["outcomes"][1]["phase_results"] =
            json!([{"phase":"Build","process_status":{"Failure":101}}]);
    });
    let log = pin(scratch.0.join("compiler.log"), log.as_bytes());
    edit_evidence(plan, |evidence| {
        evidence["mutations"][0]["compiler_failures"] = json!([{
            "mutant_name":"generated exact mutant", "log":pin_value(&log)
        }]);
    });
}

#[test]
fn compiler_invalid_is_accounted_separately_with_exact_completed_build_evidence() {
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    make_unviable(
        &mut plan,
        &scratch,
        "\n*** generated exact mutant\nerror[E0277]: generated type error\n --> crates/widget/src/lib.rs:1:1\n*** result: Failure(101)\n",
    );
    gate(&plan).unwrap();
    edit_evidence(&mut plan, |evidence| {
        evidence["mutations"][0]["compiler_failures"] = json!([]);
    });
    assert!(gate(&plan).unwrap_err().contains("no pinned compiler"));
}

#[test]
fn timeout_infrastructure_foreign_or_incomplete_build_is_not_compiler_invalid() {
    let good = "\n*** generated exact mutant\nerror[E0277]: generated type error\n --> crates/widget/src/lib.rs:1:1\n*** result: Failure(101)\n";
    for (from, to) in [
        ("generated exact mutant", "foreign mutant"),
        ("lib.rs:1:1", "lib.rs:2:1"),
        ("error[E0277]", "error: No space left on device"),
        ("*** result: Failure(101)", "*** result: Timeout"),
        ("generated type error", "internal compiler error"),
    ] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        make_unviable(&mut plan, &scratch, &good.replace(from, to));
        assert!(gate(&plan).is_err(), "{from}");
    }
    for phase in [
        json!({"phase":"Build","process_status":"Timeout"}),
        json!({"phase":"Test","process_status":{"Failure":101}}),
    ] {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        make_unviable(&mut plan, &scratch, good);
        edit_report(&mut plan, "mutations", "outcomes", |report| {
            report["outcomes"][1]["phase_results"] = json!([phase]);
        });
        assert!(gate(&plan).unwrap_err().contains("completed compiler"));
    }
}

#[test]
fn changed_source_added_source_and_replaced_pinned_report_refuse() {
    let scratch = Scratch::new();
    let plan = gated_plan(&scratch);
    let required = plan.mandatory_gates.as_ref().unwrap();
    fs::write(
        required.source_root.join("crates/widget/src/new.rs"),
        b"pub fn new() {}\n",
    )
    .unwrap();
    assert!(gate(&plan).unwrap_err().contains("source census"));
    fs::remove_file(required.source_root.join("crates/widget/src/new.rs")).unwrap();
    fs::write(
        required.source_root.join("crates/widget/src/lib.rs"),
        b"changed",
    )
    .unwrap();
    assert!(gate(&plan).unwrap_err().contains("length"));
    let scratch = Scratch::new();
    let plan = gated_plan(&scratch);
    fs::write(scratch.0.join("outcomes.json"), b"changed").unwrap();
    assert!(gate(&plan).unwrap_err().contains("length"));
}

#[test]
fn mutation_baseline_duplicates_and_foreign_identities_refuse() {
    for mode in 0..6 {
        let scratch = Scratch::new();
        let mut plan = gated_plan(&scratch);
        edit_report(&mut plan, "mutations", "outcomes", |report| match mode {
            0 => {
                report["outcomes"][0]["summary"] = json!("Failure");
            }
            1 => {
                report["outcomes"][1]["scenario"]["Mutant"]["file"] =
                    json!("crates/widget/src/foreign.rs");
            }
            2 => {
                let row = report["outcomes"][1].clone();
                report["outcomes"].as_array_mut().unwrap().push(row);
            }
            3 => {
                report["outcomes"][1]["scenario"]["Mutant"]["replacement"] = json!("different");
            }
            4 => {
                report["outcomes"][0]["phase_results"] = json!([]);
            }
            _ => {
                report["outcomes"][0]["phase_results"][1]["process_status"] = json!("Timeout");
            }
        });
        assert!(gate(&plan).is_err(), "mode {mode}");
    }
}

#[test]
fn mandatory_scope_and_byte_admission_fail_before_missing_measurement_reads() {
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    let required = plan.mandatory_gates.as_mut().unwrap();
    required.sources[0].bytes = "67108865".into();
    assert!(gate(&plan).unwrap_err().contains("source census exceeds"));
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    let required = plan.mandatory_gates.as_mut().unwrap();
    required.evidence.bytes = "1048577".into();
    fs::remove_file(&required.evidence.path).unwrap();
    assert!(gate(&plan).unwrap_err().contains("physical limit"));
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    plan.mandatory_gates
        .as_mut()
        .unwrap()
        .modules
        .push("../foreign.rs".into());
    assert!(gate(&plan).unwrap_err().contains("noncanonical"));
    let scratch = Scratch::new();
    let mut plan = gated_plan(&scratch);
    plan.mandatory_gates.as_mut().unwrap().crates.clear();
    assert!(gate(&plan).unwrap_err().contains("scope is empty"));
}
