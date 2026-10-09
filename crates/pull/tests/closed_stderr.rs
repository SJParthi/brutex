//! A degraded decode with a closed stderr finishes, and its lost line is
//! counted (r53-1, D-4413).
//!
//! The r53 audit ran `pull::csv::decode` with stderr connected to a closed pipe
//! and it panicked — "failed printing to stderr: Broken pipe (os error 32)" —
//! which the release profile turns into an abort: the decode's own notice
//! killed the backfill it described. The parent here re-runs this binary as a
//! child whose stderr is a pipe with its read end already closed; the child
//! decodes a file with a negative open interest, which writes that notice, and
//! must finish with the rows decoded and the line counted.

#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

const CHILD: &str = "BRUTEX_PULL_CLOSED_STDERR_CHILD";

#[test]
fn a_degraded_decode_with_a_closed_stderr_finishes_and_counts_the_line() {
    if std::env::var_os(CHILD).is_some() {
        let before = telemetry::unprinted();
        let body = "20221003,09:15:01,38445.65,0,-5\n20221003,09:15:02,38419.40,0,7\n";
        let rows = pull::csv::decode(body, pull::csv::Columns::TrueDataFno)
            .expect("one bad row is not a bad file");
        assert_eq!(rows.len(), 1, "the decode finished: {rows:?}");
        assert_eq!(
            telemetry::unprinted(),
            before + 1,
            "the notice the closed stream refused is counted, not lost silently"
        );
        println!("CHILD-RAN");
        return;
    }
    let (reader, writer) = std::io::pipe().expect("a pipe");
    drop(reader);
    let output = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args([
            "--exact",
            "a_degraded_decode_with_a_closed_stderr_finishes_and_counts_the_line",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .stderr(writer)
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed") && stdout.contains("CHILD-RAN"),
        "the child failed or ran nothing with a closed stderr: {:?}\n{stdout}",
        output.status
    );
}
