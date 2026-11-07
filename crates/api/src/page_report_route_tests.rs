//! `POST /page-report` (R3-14): bounded, checked, rationed and audited.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes"
)]

use super::*;

/// **A PAGE REPORT IS RATIONED AND AUDITED** (R3-14). A body past 1 KiB is a
/// 413 and an empty, foreign or control-character field a 400, each writing
/// nothing; the first [`REPORTS_PER_WINDOW`] reports of a window are written
/// where the audit stream keeps them; the rest are counted and answered 429;
/// the next window's first report says how many were kept out.
///
/// One test, because the ration is the process's one window: two tests
/// spending it at once would each see the other's reports.
#[test]
fn a_page_report_is_rationed_and_audited() {
    let mark = crate::emitted::mark();
    // A window no other caller is in: far past any clock this test runs at.
    let t = 4_102_444_800_000_u64;
    let page = "/db-page-report-test";

    let long = format!("page={page}&what={}", "x".repeat(MAX_REPORT_BYTES));
    assert_eq!(answer(&long, t).0, 413);
    assert_eq!(
        read(&format!(
            "page={page}&what={}",
            "y".repeat(MAX_REPORT_BYTES - page.len() - 11)
        )),
        Ok(Report {
            page: page.to_owned(),
            what: "y".repeat(MAX_REPORT_BYTES - page.len() - 11),
        }),
        "exactly 1 KiB is read"
    );
    for (body, field) in [
        ("what=boom", "page"),
        ("page=&what=boom", "page"),
        ("page=https%3A%2F%2Fother.example%2F&what=boom", "page"),
        ("page=%2Fdb&what=", "what"),
        ("page=%2Fdb&what=%20%20", "what"),
        ("page=%2Fdb%0A&what=boom", "page"),
        ("page=%2Fdb&what=a%07b", "what"),
    ] {
        let (status, json) = answer(body, t);
        assert_eq!(status, 400, "{body}: {json}");
        assert!(
            json.contains(&format!("{field}=")),
            "{body} names {field}: {json}"
        );
    }
    assert_eq!(
        read(&format!("page=%2F{}&what=x", "p".repeat(MAX_PAGE_BYTES))),
        Err(Refusal::Field("page", "is longer than 256 bytes"))
    );
    assert!(
        read("page=%2Fdb&what=line%0Atab%09ok").is_ok(),
        "a newline or tab in what is kept"
    );

    let body = format!("page={page}&what=TypeError%3A+x+is+undefined");
    for i in 0..REPORTS_PER_WINDOW {
        let (status, json) = answer(&body, t + i);
        assert_eq!(status, 202, "report {i}: {json}");
    }
    let before: serde_json::Value = serde_json::from_str(&answer(&body, t + 100).1).unwrap();
    assert_eq!(before["written"], false);
    let (status, _) = answer(&body, t + 101);
    assert_eq!(status, 429, "past the ration: counted, not written");
    let (status, json) = answer(&body, t + crate::logs::FAILED_LINE_WINDOW_MS + 1);
    assert_eq!(status, 202, "a new window writes again: {json}");
    let after: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(after["kept_out_total"].as_u64().unwrap() > before["kept_out_total"].as_u64().unwrap());

    assert!(telemetry::is_audited(
        telemetry::Level::Warn,
        "api.page.access",
        "page reported a failure"
    ));
    let lines: Vec<_> = crate::emitted::landed(mark, "api.page.access", "page reported a failure")
        .into_iter()
        .filter(|record| crate::emitted::says(record, "page", page))
        .collect();
    assert_eq!(
        lines.len(),
        usize::try_from(REPORTS_PER_WINDOW + 1).unwrap(),
        "the ration's reports and the next window's first, nothing else"
    );
    assert!(
        lines
            .iter()
            .all(|r| crate::emitted::says(r, "what", "TypeError: x is undefined"))
    );
    // `landed` reads newest first (`telemetry::tail`): the next window's
    // report is the first line, and it alone says how many were kept out.
    let newest = lines.first().unwrap();
    assert!(
        crate::emitted::counts(newest, "kept_out_before", 2),
        "the next window says how many were kept out: {newest:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|r| r.field("kept_out_before").is_some())
            .count(),
        1,
        "only the first line of the next window carries the count"
    );
}

/// **A REPORT THE LOG DID NOT TAKE IS NEVER ANSWERED "WRITTEN"** (R3-14
/// repair 1). `note` ignored `telemetry::emit`'s answer, so a line the log
/// dropped (a full disk, no log open) was answered `202 written` and counted
/// in `written_total`. Each of the log's four answers, and the route's own
/// ration, now earns its own status.
#[test]
fn a_report_the_log_did_not_take_is_never_answered_written() {
    let with = |log| Outcome {
        written: matches!(log, Some(telemetry::Emitted::Written)),
        log,
        written_total: 0,
        kept_out_total: 0,
    };
    assert_eq!(status_of(&with(Some(telemetry::Emitted::Written))).0, 202);
    assert_eq!(status_of(&with(None)).0, 429);
    let held = status_of(&with(Some(telemetry::Emitted::Filtered)));
    assert_eq!(held.0, 202);
    assert!(held.1.starts_with("counted, not written"), "{}", held.1);
    for failed in [
        telemetry::Emitted::Dropped,
        telemetry::Emitted::NotInstalled,
    ] {
        let (status, why) = status_of(&with(Some(failed)));
        assert_eq!(status, 503, "{failed:?}");
        assert!(why.starts_with("NOT WRITTEN"), "{why}");
    }
    // AND `note` READS THE LOG'S ANSWER rather than dropping it. Not driven
    // here: the ration is the process's one window, which the test above
    // spends (see its note). The needles are split so this file cannot match.
    let source = include_str!("page_report_route.rs");
    let body = source
        .split_once(concat!("\npub fn ", "note("))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    let asked = body.find(concat!("let log = telemetry::", "emit(&line);"));
    let read_back = body.find(concat!("let written = log.", "is_written();"));
    assert!(
        matches!((asked, read_back), (Some(a), Some(r)) if a < r),
        "note counts a report written only when the log wrote it: {body}"
    );
    assert!(
        !body.contains(concat!("let _dropped_when_", "filtered")),
        "and drops the log's answer nowhere: {body}"
    );
}

/// The variable naming the proof file of
/// [`a_report_in_a_process_with_no_log_is_answered_not_written_and_never_counted`]'s
/// child.
const NO_LOG_CHILD: &str = "BRUTEX_PAGE_REPORT_NO_LOG_CHILD";

/// **A REPORT IN A PROCESS WITH NO LOG IS ANSWERED NOT WRITTEN AND NEVER
/// COUNTED WRITTEN** (BASE-COV-API repair 3, D-8661). `note` driven whole in a
/// child of this test binary, which has its own ration window and no log: the
/// parent's log is a process singleton installed by its first use
/// (`emitted::sink`), which the child never makes. The child proves it ran by
/// the file its parent reads, never by a print. Extremes: the window's first
/// report with no log (`503 NOT WRITTEN`, `written` false); its
/// `written_total` (`0`: nothing was written); the window's last admitted
/// report (`503`, the total still `0`); the report past the ration (`429`,
/// kept out before the log is asked); a report of exactly 1 KiB (read, then
/// not written); a body past 1 KiB (`413`, the log never asked); an empty
/// page (`400`, the same); the next window's first report (`503` again: a new
/// window makes no log); and, at once, the ration spent with no log (every
/// admitted report `503` and none counted written), and the next window's
/// report with no log carrying the reports kept out (`503`, `kept_out_total`
/// `1`); out of the box, thirty-one reports in one millisecond (thirty
/// admitted, one kept out: the ration counts reports, not milliseconds).
#[test]
fn a_report_in_a_process_with_no_log_is_answered_not_written_and_never_counted() {
    if let Some(proof) = std::env::var_os(NO_LOG_CHILD) {
        let t = 4_102_444_800_000_u64;
        let body = "page=%2Fno-log&what=boom";
        let mut said: Vec<(u16, String)> = Vec::new();
        for _ in 0..=REPORTS_PER_WINDOW {
            said.push(answer(body, t));
        }
        let next = t + crate::logs::FAILED_LINE_WINDOW_MS + 1;
        said.push(answer(body, next));
        // EXACTLY 1 KiB: the page unencoded, as the test above writes it, so
        // the body is `page=` + page + `&what=` + what, 11 bytes besides.
        let page = "/no-log";
        said.push(answer(
            &format!(
                "page={page}&what={}",
                "y".repeat(MAX_REPORT_BYTES - page.len() - 11)
            ),
            next,
        ));
        said.push(answer(
            &format!("page=%2Fno-log&what={}", "x".repeat(MAX_REPORT_BYTES)),
            t,
        ));
        said.push(answer("page=&what=boom", t));
        let text = said
            .iter()
            .map(|(status, json)| format!("{status} {json}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(proof, text).expect("the child's proof");
        return;
    }
    let dir = crate::scratch::path("page-report-no-log");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    let proof = dir.join("child-ran");
    let _ = crate::isolated::rerun(
        "page_report_route::tests::a_report_in_a_process_with_no_log_is_answered_not_written_and_never_counted",
        &[(NO_LOG_CHILD, proof.as_os_str())],
    );
    let text = std::fs::read_to_string(&proof).expect("the child ran and wrote its proof");
    let said: Vec<(u16, serde_json::Value)> = text
        .lines()
        .map(|line| {
            let (status, json) = line.split_once(' ').expect("a status and its JSON");
            (
                status.parse().expect("a status"),
                serde_json::from_str(json).expect("JSON"),
            )
        })
        .collect();
    let admitted = usize::try_from(REPORTS_PER_WINDOW).expect("thirty");
    assert_eq!(said.len(), admitted + 5, "{text}");
    for (n, (status, json)) in said.iter().take(admitted).enumerate() {
        assert_eq!(*status, 503, "report {n}: {json}");
        assert_eq!(json["written"], false, "report {n}: {json}");
        assert_eq!(json["written_total"], 0, "report {n}: {json}");
        assert!(
            json["why"]
                .as_str()
                .is_some_and(|why| why.starts_with("NOT WRITTEN")),
            "report {n}: {json}"
        );
    }
    let (kept_status, kept) = &said[admitted];
    assert_eq!(*kept_status, 429, "past the ration: {kept}");
    assert_eq!(kept["kept_out_total"], 1, "{kept}");
    let (next_status, next) = &said[admitted + 1];
    assert_eq!(*next_status, 503, "a new window makes no log: {next}");
    assert_eq!(next["written"], false, "{next}");
    assert_eq!(next["written_total"], 0, "{next}");
    assert_eq!(next["kept_out_total"], 1, "{next}");
    let (whole_status, whole) = &said[admitted + 2];
    assert_eq!(
        *whole_status, 503,
        "exactly 1 KiB is read, then not written: {whole}"
    );
    assert_eq!(whole["written_total"], 0, "{whole}");
    assert_eq!(said[admitted + 3].0, 413, "{text}");
    assert_eq!(said[admitted + 4].0, 400, "{text}");
}
