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
