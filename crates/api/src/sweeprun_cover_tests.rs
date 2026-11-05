//! What no other test of `sweeprun.rs` reached, each row driven by its own
//! input and asserted on what the code answers there (BASE-COV-API-AS3).
//! Every store is an empty scratch folder and every log line is invented: no
//! bar is read and no vendor's row is written.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use axum::http::StatusCode;
use cli::audited_range_command::StrictConfig;

/// Names a child's scratch root (`crate::isolated`).
const CHILD: &str = "BRUTEX_TEST_SWEEP_COVER_CHILD";
/// A build stamp of the right shape; no commit has it.
const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";
/// One month, which every command reads.
const SPAN: &str = r#""from_year":2025,"from_month":5,"to_year":2025,"to_month":5"#;

/// A fresh scratch directory of this module's own.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = crate::scratch::path(&format!("sweep-cover-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A site over `root`'s `masters` and `store`, neither made here.
fn site_in(root: &std::path::Path) -> crate::server::Loaded {
    std::sync::Arc::new(crate::server::Site::load(
        &root.join("masters"),
        &root.join("store"),
    ))
}

/// The strict command's body, `extra` members written before its end.
fn strict_body(extra: &str) -> String {
    format!(
        r#"{{"command":"audit-audited-range","feed":"zerodha","underlying":"NIFTY","rung":"5min",{SPAN},"min_hits":500{extra}}}"#
    )
}

/// An ordinary engine command's body.
fn ordinary_body() -> String {
    format!(
        r#"{{"command":"audit-range","feed":"zerodha","underlying":"NIFTY","rung":"15min",{SPAN},"min_hits":500}}"#
    )
}

/// A strict configuration over a receipt folder of this module's own.
fn config() -> StrictConfig {
    let root = scratch("receipts");
    StrictConfig::from_values(
        Some(root.into()),
        Some("33554432".into()),
        Some("1000000".into()),
    )
    .expect("explicit physical configuration")
}

/// A tail that read every record it holds, with no fault.
fn tail(records: Vec<telemetry::Record>) -> telemetry::Tail {
    telemetry::Tail {
        records,
        bytes_read: 0,
        files_read: 1,
        malformed: 0,
        partial_tail: false,
        hit_scan_cap: false,
        reached_oldest: true,
        errors: Vec::new(),
        missing: Some(0),
    }
}

/// A lifecycle record of the CLI's: `message`, `run` and `command`, and a
/// `phase` when one is given, written at `at`.
fn lifecycle(
    message: &str,
    run: u64,
    phase: Option<&str>,
    command: &str,
    at: i64,
) -> telemetry::Record {
    telemetry::Record {
        seq: 1,
        run,
        at_unix_millis: at,
        at_utc: "2026-10-05T03:31:18.967Z".to_owned(),
        level: telemetry::Level::Info,
        target: "cli.lifecycle".to_owned(),
        message: message.to_owned(),
        fields: [("command", Some(command)), ("phase", phase)]
            .into_iter()
            .filter_map(|(key, value)| {
                value.map(|value| (key.to_owned(), telemetry::OwnedValue::Str(value.to_owned())))
            })
            .collect(),
        cut: false,
        dropped_fields: 0,
    }
}

/// An external observation that says only what is given.
fn observation(
    at_millis: Option<i64>,
    unterminated: Option<(u64, String)>,
    body: &str,
) -> ExternalObservation {
    ExternalObservation {
        at_millis,
        uncertain: false,
        in_flight: false,
        launch_clear: true,
        unterminated,
        quiet: None,
        body: body.to_owned(),
    }
}

/// **THE STATUS DOOR ANSWERS EACH OBSERVATION BY WHAT IT CAN PROVE**
/// (BASE-COV-API-AS3). An external event newer than this process's finished
/// run answers in its place; a lifecycle event naming no command is no
/// command; a capped window concludes nothing; "no sweep has run" names no
/// command when none is named, names an older command once however often it
/// started, and is not said at all over a ledger path that cannot be asked
/// about; a clean lifecycle beside a damaged legacy tail is unknown; an
/// activity tail that is damaged makes the marker uncertain; and every
/// marker short of a usable receipt is `unknown`. A marker no sweep named is
/// no unterminated command; an invocation of another origin is no CLI
/// command that ended without its terminal; and a walk that was not kept
/// says so in its own words.
///
/// Extremes: singles — an external event one millisecond after the local
/// finish (and, the control, at it); a command field that is empty, and one
/// that is a number; a capped window; a quiet log naming no command; a marker
/// with no phase; a marker at run zero, running, completed and refused; a
/// walk that was not kept; combinations — a running marker of a command that
/// is no sweep, and a refused one; two older starts of one command beside an
/// active one (the command named once); a clean lifecycle beside a damaged
/// legacy tail; a browser invocation found under a CLI marker's id; out of
/// the box — a FILE where the results folder should be, so the ledger path
/// cannot even be asked about.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one door, every observation it can be handed, each asserted"
)]
fn the_status_door_answers_each_observation_by_what_it_can_prove() {
    let mut local = Progress::started("zerodha", "NIFTY", (2025, 1), (2025, 1), None, 1_000_000, 9);
    local.finished_micros = Some(2_000_000);
    assert_eq!(
        observed_status(Some(&local), observation(Some(2_001), None, "EXTERNAL")),
        "EXTERNAL"
    );
    assert_eq!(
        observed_status(Some(&local), observation(Some(2_000), None, "EXTERNAL")),
        format!(r#"{{"running":{}}}"#, local.to_json())
    );
    let named = |command: telemetry::OwnedValue| {
        let mut record = lifecycle("command started", 5, None, "x", 1);
        record.fields = vec![("command".to_owned(), command)];
        Logged::of(&record).map(|logged| logged.command)
    };
    assert_eq!(named(telemetry::OwnedValue::Str(String::new())), None);
    assert_eq!(named(telemetry::OwnedValue::Uint(7)), None);
    assert_eq!(
        named(telemetry::OwnedValue::Str("census".to_owned())),
        Some("census".to_owned())
    );
    let mut capped = tail(vec![lifecycle(
        "command started",
        5,
        Some("running"),
        "census",
        1,
    )]);
    capped.hit_scan_cap = true;
    assert_eq!(
        Quiet::of(&capped, None),
        None,
        "a capped window concludes nothing"
    );
    let root = scratch("no-sweep");
    let started = |command: &str, run: u64| Logged {
        command: command.to_owned(),
        message: "command started".to_owned(),
        run,
        at: Stamp {
            millis: 1,
            utc: "2026-10-05T03:31:18.967Z".to_owned(),
        },
    };
    let quiet = |unfinished: Vec<Logged>| Quiet {
        events: 3,
        oldest: None,
        unfinished,
        newest: None,
    };
    let none = no_sweep_has_run(&root, &quiet(Vec::new())).expect("no ledger");
    assert!(
        none.contains("No sweep has run; the retained CLI lifecycle log names no command."),
        "{none}"
    );
    let older = vec![
        started("gdfl-import", 3),
        started("census", 2),
        started("census", 1),
    ];
    let several = no_sweep_has_run(&root, &quiet(older)).expect("no ledger");
    assert!(
        several.contains("2 older start(s) logged no finish either (census); none is a sweep."),
        "{several}"
    );
    let blocked = scratch("no-sweep-blocked");
    std::fs::write(blocked.join("results"), b"a file where a folder goes").expect("a file");
    assert_eq!(no_sweep_has_run(&blocked, &quiet(Vec::new())), None);
    let mut damaged = tail(Vec::new());
    damaged.errors = vec!["an unreadable file".to_owned()];
    let legacy = unmarked(&tail(Vec::new()), &damaged);
    assert!(legacy.uncertain);
    assert!(!legacy.launch_clear);
    assert_eq!(
        legacy.quiet, None,
        "nothing is concluded from a damaged tail"
    );
    assert!(
        legacy.body.contains("external sweep log is incomplete"),
        "{}",
        legacy.body
    );
    let now = 10_000_000;
    let sweep = |message: &str, run: u64, phase: Option<&str>| {
        lifecycle(message, run, phase, "sweep-stored", now - 1_000)
    };
    let uncertain = marked(&sweep("command started", 5, Some("running")), &damaged, now);
    assert!(uncertain.uncertain);
    assert_eq!(uncertain.at_millis, Some(now - 1_000));
    assert!(uncertain.body.contains("external sweep log is incomplete"));
    for (case, said) in [
        ("no phase", sweep("command started", 5, None)),
        (
            "running at run 0",
            sweep("command started", 0, Some("running")),
        ),
        (
            "running, no sweep",
            lifecycle("command started", 5, Some("running"), "census", now - 1_000),
        ),
        (
            "completed at run 0",
            sweep("command finished", 0, Some("completed")),
        ),
        (
            "refused at run 0",
            sweep("command finished", 0, Some("refused")),
        ),
        (
            "refused, no sweep",
            lifecycle(
                "command finished",
                5,
                Some("refused"),
                "census",
                now - 1_000,
            ),
        ),
    ] {
        let observed = marked(&said, &tail(Vec::new()), now);
        assert!(observed.uncertain, "{case}");
        assert!(!observed.launch_clear, "{case}");
        assert!(
            observed.body.contains(r#""status":"unknown""#),
            "{case}: {}",
            observed.body
        );
    }
    let id = cli::operation_audit::ID_BASE + 3;
    let open = sweep("command started", id, Some("running"));
    assert_eq!(unterminated_marker(&open, false), None, "no sweep named it");
    assert_eq!(
        unterminated_marker(&open, true),
        Some((id, "sweep-stored".to_owned()))
    );
    let store = scratch("ended-without-terminal");
    let browser = cli::operation_audit::begin(
        &store,
        cli::operation_audit::Origin::Browser,
        "sweep-stored",
    )
    .expect("a browser invocation");
    let by_cli =
        cli::operation_audit::begin(&store, cli::operation_audit::Origin::Cli, "sweep-stored")
            .expect("a CLI invocation");
    let under = |id: u64| observation(None, Some((id, "sweep-stored".to_owned())), "");
    assert_eq!(ended_without_terminal(&store, &under(browser.id())), None);
    assert_eq!(
        ended_without_terminal(&store, &under(by_cli.id())),
        Some(by_cli.id())
    );
    drop((browser, by_cli));
    assert_eq!(Answered::WalkNotKept.word(), "walk-not-kept");
    let answer = LogAnswer {
        answered: Answered::WalkNotKept,
        bytes_read: 3,
        walked_at_unix_millis: 4,
        walks: 5,
        from_memory: 6,
    };
    let json: serde_json::Value = serde_json::from_str(&cli_log_json(Some(&answer))).expect("JSON");
    assert_eq!(json["answered_from"], "walk-not-kept");
    assert!(
        json["why"]
            .as_str()
            .expect("words")
            .contains("the walk was not remembered"),
        "{json}"
    );
    for dir in [root, blocked, store] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// **A FINISHER WITH NO AUDIT RUNS ITS WORK BARE, AND A FINISH WITH NEITHER
/// A REPORT NOR A REFUSAL CLOSES ITS AUDIT AS FAILED** (BASE-COV-API-AS3).
///
/// Extremes: singles — a finisher armed with no durable invocation; a
/// finished run holding neither a report nor a refusal; combinations — that
/// empty finish under a durable invocation, read back from the journal as
/// `Failed`; out of the box — a run settled with nothing to say at all.
#[test]
fn a_finisher_with_no_audit_runs_bare_and_an_empty_finish_closes_failed() {
    let bare_root = scratch("finisher-bare");
    let bare = site_in(&bare_root);
    let guard = TaskFinisher::new(std::sync::Arc::clone(&bare));
    let done = guard.conduct(|| {
        let mut progress = Progress::started("zerodha", "NIFTY", (2025, 1), (2025, 1), None, 1, 2);
        settle(&mut progress, "an invented report".to_owned(), 5);
        progress
    });
    assert_eq!(done.report.as_deref(), Some("an invented report"));
    guard.finish(done);
    let held = bare
        .sweep
        .lock()
        .expect("the slot")
        .clone()
        .expect("installed");
    assert_eq!(held.report.as_deref(), Some("an invented report"));
    let audited_root = scratch("finisher-neither");
    let audited = site_in(&audited_root);
    std::fs::create_dir_all(&audited.store_root).expect("a store");
    let audit = reserve_invocation(&audited, "sweep").expect("a durable invocation");
    let id = audit.id();
    let guard = TaskFinisher::audited(std::sync::Arc::clone(&audited), audit);
    let mut neither = Progress::started("zerodha", "NIFTY", (2025, 1), (2025, 1), None, 1, id);
    neither.finished_micros = Some(9);
    assert_eq!(completion_audit(&neither).outcome, "invalid");
    guard.finish(neither);
    let saved = cli::operation_audit::read(&audited.store_root, id)
        .expect("read exact")
        .expect("saved");
    assert_eq!(saved.phase, cli::operation_audit::Phase::Failed);
    let _ = std::fs::remove_dir_all(bare_root);
    let _ = std::fs::remove_dir_all(audited_root);
}

/// **AN ADMISSION OVER A STORE THAT IS NOT THERE IS REFUSED BY NAME**
/// (BASE-COV-API-AS3): no durable invocation can start and no lease can be
/// taken, each said with a 503, and nothing is created.
///
/// Extremes: singles — a store root that is not there; combinations — the
/// same two refusals over a FILE where the store should be; out of the box —
/// that file, which no refusal may remove or replace.
#[test]
fn an_admission_over_a_store_that_is_not_there_is_refused_by_name() {
    let root = scratch("store-not-there");
    let gone = site_in(&root);
    let file = scratch("store-is-a-file");
    std::fs::write(file.join("store"), b"a file where a store goes").expect("a file");
    let flat = site_in(&file);
    for site in [&gone, &flat] {
        let why = reserve_invocation(site, "sweep")
            .err()
            .expect("no store, no durable invocation");
        assert_eq!(why.status(), StatusCode::SERVICE_UNAVAILABLE);
        let said = why.why();
        assert!(
            said.starts_with("required durable invocation audit could not start: "),
            "{said}"
        );
        assert!(said.ends_with("No engine work was started."), "{said}");
        let lease = claim_execution(site, false)
            .err()
            .expect("no store, no lease");
        assert_eq!(lease.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            lease.why().contains("execution lease is unavailable"),
            "{}",
            lease.why()
        );
    }
    assert!(!gone.store_root.exists(), "nothing was created");
    assert_eq!(
        std::fs::read(&flat.store_root).expect("still the file"),
        b"a file where a store goes"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(file);
}

/// **A COMMAND THIS CONDUCTOR DOES NOT RUN, AND A STRICT REQUEST OF ANOTHER
/// COMMAND, ARE REFUSED BY NAME; A VALIDATION WORD IS ONE OF TWO VALUES**
/// (BASE-COV-API-AS3). Declared Boolean research reached past its prepared
/// dispatch is refused here; the strict audit asked of an ordinary command
/// refuses, through its conductor too, and that command has no strict knob
/// context; and every spelling of `validate` the strict command accepts
/// reaches the engine as `1` or `0`.
///
/// Extremes: singles — Boolean research handed to the ordinary conductor; a
/// strict request of `audit-range`; `validate` as JSON `true`, `"yes"`,
/// `"ON"`, `"1"`, JSON `false` and `"off"`; combinations — the strict
/// conductor with a whole configuration over an ordinary command; the strict
/// command with and without a validation word; out of the box — `"ON"` in
/// capitals, a switch word no lower-case table holds as written.
#[test]
fn a_command_this_conductor_does_not_run_and_a_strict_knob_are_answered_by_name() {
    let boolean = format!(
        r#"{{"command":"{}","feed":"zerodha","symbols":["NSE-NIFTY","NSE-RELIANCE"],"from_year":2025,"from_month":4,"to_year":2025,"to_month":5,"later_from_year":2025,"later_from_month":6,"later_to_year":2025,"later_to_month":8,"bits":"52,53","horizon_bars":"5","max_points":"5","batch_programs":"2","node_allowance":"128","batch_allowance":"1","expected_policy_digest":"0707070707070707070707070707070707070707070707070707070707070707"}}"#,
        crate::booleanlaunch::COMMAND
    );
    let research = command_from(&boolean).expect("Boolean research");
    let done = conduct_command(&research, 7, 3);
    assert_eq!(
        done.refusal.as_deref(),
        Some("refused: declared Boolean research requires its prepared audited browser dispatch")
    );
    assert!(done.report.is_none());
    assert_eq!(done.finished_micros, Some(7));
    let range = command_from(&ordinary_body()).expect("an ordinary command");
    let root = scratch("strict-of-another");
    let why = strict_request(&range, 3, &root)
        .err()
        .expect("not the strict command");
    assert_eq!(
        why,
        "strict input configuration requires the explicit audit-audited-range command"
    );
    let conducted = conduct_strict_command(&range, 5, 6, &config(), &root);
    assert_eq!(
        conducted.refusal.as_deref(),
        Some(&*format!("refused: {why}"))
    );
    assert!(strict_knob_context(&range).is_none());
    let strict = command_from(&strict_body("")).expect("the strict command");
    assert_eq!(
        strict_knob_context(&strict).map(|asked| asked.knobs),
        Some(Vec::new())
    );
    for (written, value) in [
        ("true", "1"),
        ("\"yes\"", "1"),
        ("\"ON\"", "1"),
        ("\"1\"", "1"),
        ("false", "0"),
        ("\"off\"", "0"),
    ] {
        let asked =
            command_from(&strict_body(&format!(r#","validate":{written}"#))).expect(written);
        let Command::AuditAuditedRange { knobs, .. } = asked else {
            panic!("{written}: the strict command")
        };
        assert_eq!(knobs, [("BRUTEX_VALIDATE", value.to_owned())], "{written}");
    }
    let _ = std::fs::remove_dir_all(root);
}

/// Runs `test` again in a child of this binary with `env`, its scratch root
/// named, and requires the child's proof that its half ran.
fn in_a_child(test: &str, root: &std::path::Path, env: &[(&str, &std::ffi::OsStr)]) {
    let mut named = vec![(CHILD, root.as_os_str())];
    named.extend_from_slice(env);
    let _ = crate::isolated::rerun(test, &named);
    assert_eq!(
        std::fs::read(root.join("child-ran")).ok().as_deref(),
        Some(&b"ran"[..]),
        "the child ran {test}"
    );
}

/// The checksum configuration a child reads from its environment, over a
/// receipt folder in `root`.
fn checksum_env(root: &std::path::Path) -> std::path::PathBuf {
    let receipts = root.join("receipts");
    std::fs::create_dir_all(&receipts).expect("a receipt folder");
    receipts
}

/// **A PRESS WHOSE START NO MONITOR COULD SEE IS REFUSED ON EVERY DOOR, AN
/// ADMISSION MEETING A SWEEP THE CLI LOG SHOWS RUNNING IS REFUSED, AND A
/// STATUS READ THE POOL CANNOT TAKE IS UNKNOWN** (BASE-COV-API-AS3).
///
/// In a child with no telemetry sink, over its own store: a run, a descent
/// and an engine command each pass every admission step and then cannot
/// write the attempt marker the browser's monitor waits for, so each is
/// refused with a 503, its durable invocation closed `Refused` and the slot
/// left empty. A CLI log that shows a sweep running then refuses the next
/// admission; and with the detail pool full, the status read is unavailable,
/// never an idle answer.
///
/// Extremes: singles — no sink, on each of the three doors; a running sweep
/// in the CLI log; a full detail pool; combinations — all three doors
/// refused in turn over one store, three journal rows all `Refused`; the
/// running sweep found by the admission that holds the lease; out of the
/// box — a process with nowhere at all to write its log.
#[test]
fn a_press_no_monitor_could_see_or_that_meets_a_running_sweep_is_refused() {
    if let Some(root) = std::env::var_os(CHILD) {
        let root = std::path::PathBuf::from(root);
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(unseen_presses(&root));
        std::fs::write(root.join("child-ran"), b"ran").expect("the child's proof");
        return;
    }
    let root = scratch("unseen");
    let store = root.join("store");
    std::fs::create_dir_all(&store).expect("the child's store");
    in_a_child(
        "sweeprun::cover_tests::a_press_no_monitor_could_see_or_that_meets_a_running_sweep_is_refused",
        &root,
        &[("BRUTEX_STORE", store.as_os_str())],
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The child half of the test above: no sink is installed in this process.
async fn unseen_presses(root: &std::path::Path) {
    assert!(
        telemetry::global().is_none(),
        "premise: no sink in this child"
    );
    let site = site_in(root);
    let store = root.join("store");
    let body = |extra: &str| format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},{extra}}}"#);
    for (door, (status, _, answer)) in [
        (
            "run",
            run_with(&site, &body(r#""rungs":["5min"]"#), Some(STAMP)),
        ),
        (
            "descend",
            descend_with(
                &site,
                &body(r#""rung":"15min","max_points":20,"top":25"#),
                Some(STAMP),
            ),
        ),
        (
            "command",
            command_with(&site, &ordinary_body(), Some(STAMP)),
        ),
    ] {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{door}: {answer}");
        assert!(answer.contains(r#""accepted":false"#), "{door}: {answer}");
        assert!(
            answer.contains("no telemetry sink is installed"),
            "{door}: {answer}"
        );
    }
    assert!(
        site.sweep.lock().expect("the slot").is_none(),
        "no press took the slot"
    );
    let rows = cli::operation_audit::page(&store, None, 8).expect("the journal reads");
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert!(
        rows.iter()
            .all(|row| row.phase == cli::operation_audit::Phase::Refused),
        "{rows:?}"
    );
    let cli_log = store.join("logs").join("cli");
    std::fs::create_dir_all(&cli_log).expect("the CLI's log folder");
    {
        let sink = telemetry::Sink::open(&telemetry::Config::new(&cli_log)).expect("the CLI's log");
        let started = telemetry::Event::info("cli.lifecycle", "command started")
            .with("phase", "running")
            .with("command", "sweep-stored");
        assert_eq!(sink.emit_for_run(61, &started), telemetry::Emitted::Written);
    }
    let elsewhere = claim_execution(&site, false)
        .err()
        .expect("a sweep runs elsewhere");
    assert_eq!(elsewhere.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        elsewhere.why().contains("still reports activity"),
        "{}",
        elsewhere.why()
    );
    let crowd = crate::detail::crowd(crate::detail::Named::Detail);
    let (status, _, answer) = run_json(
        axum::extract::State(std::sync::Arc::clone(&site)),
        "/backtest/run.json".parse().expect("a target"),
    )
    .await;
    drop(crowd);
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{answer}");
    assert!(
        answer.contains("external sweep status is unavailable"),
        "{answer}"
    );
    assert!(answer.contains(r#""status":"unknown""#), "{answer}");
}

/// **A STORE THAT CANNOT BE NAMED READS NO CLI LOG, AND THE STRICT COMMAND
/// OVER IT IS REFUSED WITH THE STORE'S OWN REASON** (BASE-COV-API-AS3).
///
/// In a child whose `BRUTEX_STORE` is set but empty, with a whole strict
/// configuration: an admission takes its site's own lease and, with no store
/// named, walks no CLI log; and the strict command, its configuration read
/// whole, is refused for the store, never run over a guessed one.
///
/// Extremes: singles — a store variable set but empty; combinations — a
/// whole strict configuration beside a store that cannot be named; out of
/// the box — an empty value where a folder is named.
#[test]
fn a_store_that_cannot_be_named_reads_no_cli_log_and_refuses_the_strict_command() {
    if let Some(root) = std::env::var_os(CHILD) {
        let root = std::path::PathBuf::from(root);
        unnamed_store(&root);
        std::fs::write(root.join("child-ran"), b"ran").expect("the child's proof");
        return;
    }
    let root = scratch("unnamed-store");
    std::fs::create_dir_all(root.join("store")).expect("the site's own store");
    let receipts = checksum_env(&root);
    in_a_child(
        "sweeprun::cover_tests::a_store_that_cannot_be_named_reads_no_cli_log_and_refuses_the_strict_command",
        &root,
        &[
            ("BRUTEX_STORE", std::ffi::OsStr::new("")),
            ("BRUTEX_CHECKSUM_RECEIPTS", receipts.as_os_str()),
            (
                "BRUTEX_CHECKSUM_MAX_BYTES",
                std::ffi::OsStr::new("33554432"),
            ),
            (
                "BRUTEX_CHECKSUM_MAX_RECORDS",
                std::ffi::OsStr::new("1000000"),
            ),
        ],
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The child half of the test above.
fn unnamed_store(root: &std::path::Path) {
    assert!(
        crate::server::store_dir().is_err(),
        "premise: no store is named"
    );
    let site = site_in(root);
    assert!(
        claim_execution(&site, false).is_ok(),
        "the site's own lease, no log read"
    );
    let strict = command_from(&strict_body("")).expect("the strict command");
    let done = conduct_command(&strict, 7, 3);
    let refusal = done.refusal.as_deref().expect("refused");
    assert!(
        refusal.starts_with("refused: BRUTEX_STORE is set but empty"),
        "{refusal}"
    );
    assert!(done.report.is_none());
}

/// **A STRICT COMMAND WITH ITS WHOLE CONFIGURATION IS CONDUCTED BY THE STRICT
/// AUDIT, CALLED AND PRESSED** (BASE-COV-API-AS3).
///
/// In a child with a sink, a named store and a whole strict configuration:
/// the ordinary conductor hands the strict command to the strict audit with
/// the configuration it read (which, with no stored source, refuses in its
/// own words, never the missing configuration's); and pressed, the accepted
/// command's task runs the strict audit to a refusal that the slot keeps.
///
/// Extremes: singles — the configuration read from the environment; an
/// accepted strict press; combinations — the press's task run to its end
/// under a runtime shut down only after it; out of the box — a strict audit
/// over a store with no source at all.
#[test]
fn a_strict_command_with_its_configuration_is_conducted_by_the_strict_audit() {
    if let Some(root) = std::env::var_os(CHILD) {
        let root = std::path::PathBuf::from(root);
        configured(&root);
        std::fs::write(root.join("child-ran"), b"ran").expect("the child's proof");
        return;
    }
    let root = scratch("configured");
    let store = root.join("store");
    std::fs::create_dir_all(&store).expect("the child's store");
    let receipts = checksum_env(&root);
    in_a_child(
        "sweeprun::cover_tests::a_strict_command_with_its_configuration_is_conducted_by_the_strict_audit",
        &root,
        &[
            ("BRUTEX_STORE", store.as_os_str()),
            ("BRUTEX_CHECKSUM_RECEIPTS", receipts.as_os_str()),
            (
                "BRUTEX_CHECKSUM_MAX_BYTES",
                std::ffi::OsStr::new("33554432"),
            ),
            (
                "BRUTEX_CHECKSUM_MAX_RECORDS",
                std::ffi::OsStr::new("1000000"),
            ),
        ],
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The child half of the test above.
fn configured(root: &std::path::Path) {
    telemetry::install(&telemetry::Config::new(root.join("logs"))).expect("this child's sink");
    let site = site_in(root);
    let strict = command_from(&strict_body("")).expect("the strict command");
    let done = conduct_command(&strict, 7, 3);
    let refusal = done
        .refusal
        .as_deref()
        .expect("no stored source, so refused");
    assert!(refusal.starts_with("refused: "), "{refusal}");
    assert!(
        !refusal.contains("strict input configuration is unavailable"),
        "{refusal}"
    );
    let config = StrictConfig::from_env().expect("the child's configuration");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime");
    let (status, _, answer) = {
        let _entered = runtime.enter();
        command_with_configuration(&site, &strict_body(""), Some(STAMP), || Ok(config))
    };
    runtime.shutdown_timeout(std::time::Duration::from_secs(120));
    assert_eq!(status, StatusCode::ACCEPTED, "{answer}");
    let held = site
        .sweep
        .lock()
        .expect("the slot")
        .clone()
        .expect("the press");
    assert!(!held.in_flight(), "the task ran to its end");
    assert!(held.report.is_none());
    let why = held.refusal.as_deref().expect("refused");
    assert!(why.starts_with("refused: "), "{why}");
}

/// One CLI lifecycle line as the CLI's sink writes it.
fn lifecycle_line(seq: u64, run: u64, ms: i64, message: &str, command: &str) -> String {
    format!(
        "{{\"seq\":{seq},\"run\":{run},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{ms},\
         \"level\":\"info\",\"target\":\"cli.lifecycle\",\"msg\":\"{message}\",\
         \"fields\":{{\"phase\":\"running\",\"command\":\"{command}\"}}}}\n"
    )
}

/// **A RE-WALK THAT NO LONGER ENDS ON THE MARKER KEEPS THE WHOLE WINDOW'S
/// VERDICT** (BASE-COV-API-AS3).
///
/// A sweep's marker is the CLI log's newest line and a line that is no
/// record lies behind it, so the window is damaged; a re-walk that stops at
/// the marker is clean and is adopted. Here the log is swapped, between the
/// walk and its re-walk, for an empty log and then for one a newer line has
/// grown: neither re-walk ends on the marker, and each time the whole
/// window's verdict, damage and all, is kept. The swaps are a race the
/// search repeats until it is seen, each case on its own.
///
/// Extremes: singles — the marker newest with damage behind it (narrowed);
/// combinations — the log emptied between the walks; the log grown by a
/// newer line between the walks; out of the box — the log's file replaced
/// under its reader, again and again.
#[test]
fn a_rewalk_that_no_longer_ends_on_the_marker_keeps_the_whole_windows_verdict() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let dir = scratch("rewalk");
    let current = telemetry::current_path(&dir);
    let next = dir.join("next.tmp");
    let marked = format!(
        "not a record\n{}",
        lifecycle_line(2, 7, 2_000, "command started", "sweep-stored")
    );
    let grown = format!(
        "{marked}{}",
        lifecycle_line(3, 8, 3_000, "a newer line", "census")
    );
    std::fs::write(&current, &marked).expect("the log");
    let (narrow, found) = newest_sweep_marker(&dir);
    assert_eq!((found, narrow.malformed), (Some(0), 0), "premise: narrowed");
    for (case, other) in [("emptied", String::new()), ("grown", grown)] {
        let stop = AtomicBool::new(false);
        let kept = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    for content in [&marked, &other] {
                        std::fs::write(&next, content).expect("the next log");
                        std::fs::rename(&next, &current).expect("swapped in");
                    }
                }
            });
            let kept = (0..200_000)
                .map(|_| newest_sweep_marker(&dir))
                .find(|(tail, found)| (*found, tail.malformed) == (Some(0), 1));
            stop.store(true, Ordering::Relaxed);
            kept
        });
        let (whole, _) = kept.unwrap_or_else(|| panic!("{case}: the swap was never seen"));
        assert_eq!(whole.records.len(), 1, "{case}: the whole window");
        assert_eq!(whole.malformed, 1, "{case}: its damage kept");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
