#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): the api child's `StdioGate` over its
//! real stdin and stdout, which only a child process of its own can give it
//! (this binary run again with a pipe for stdin), and `await_go` with the
//! production wait. Every run id is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::io::Write as _;

use super::*;

/// The variable a child of the gate test reads its case from.
const CASE: &str = "BRUTEX_START_GATE_COV_CASE";

/// This test's name, which a child runs `--exact`.
const GATE: &str =
    "gdfl_run::start::cov_tests::the_api_childs_gate_says_claimed_and_hears_its_stdin";

/// A thread stack no host can map, 1 PiB (2^50 bytes, page-aligned on every
/// host): the size `std` gives every thread spawned without a size of its
/// own when `RUST_MIN_STACK` names it, so a child started with it cannot
/// spawn the gate's reader (the host refuses the mapping, `EAGAIN`).
const UNMAPPABLE_STACK: &str = "1125899906842624";

/// **THE API CHILD'S GATE SAYS `claimed` ON STDOUT AND HEARS ITS STDIN**:
/// each case is a child of this binary with a pipe for stdin: a `go` line is
/// `Go` (ended `\r\n`, or by the end of the input with no line end at
/// all), any other line (`GO` too) and a closed stdin are `Release`, and
/// stdout carries `claimed run=<id> kind=<kind>` before the wait. Nothing a
/// parent writes in time is `Silent` (a wait of a few milliseconds, or of
/// none at all, the pipe left open).
/// A gate whose reader thread cannot be spawned (the host refuses its stack)
/// releases at once, the `go` on its stdin unread: a run never starts on an
/// answer nobody read.
#[test]
fn the_api_childs_gate_says_claimed_and_hears_its_stdin() {
    if let Ok(case) = std::env::var(CASE) {
        gate_case(&case);
    } else {
        child("go", Some("go\n"));
        child("release", Some("release\n"));
        child("closed", None);
        child("silent", Some(""));
        child("go-crlf", Some("go\r\n"));
        child("go-unterminated", Some("go"));
        child("upper-case-first", Some("GO\ngo\n"));
        child("silent-zero", Some(""));
        unspawnable_child();
    }
}

/// One case, in the child started for it.
fn gate_case(case: &str) {
    let mut gate = StdioGate;
    gate.claimed(41, ImportKind::Stocks);
    let heard = match case {
        "silent" => gate.wait(Duration::from_millis(20)),
        "silent-zero" => gate.wait(Duration::ZERO),
        _ => gate.wait(Duration::from_secs(20)),
    };
    let want = match case {
        "go" | "go-crlf" | "go-unterminated" => Gate::Go,
        "silent" | "silent-zero" => Gate::Silent,
        _ => Gate::Release,
    };
    assert_eq!(heard, want, "{case}");
}

/// The command that runs [`GATE`] alone in a child for `case`, its stdout
/// and stderr kept.
fn gate_command(case: &str) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", GATE, "--nocapture", "--test-threads=1"])
        .env(CASE, case)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    command
}

/// Asserts the child of `case` passed its one test and said its claim.
fn passed(case: &str, output: &std::process::Output) {
    let said = format!(
        "{case}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{said}");
    assert!(said.contains("claimed run=41 kind=stocks"), "{said}");
    assert!(said.contains("1 passed"), "{said}");
}

/// Runs [`GATE`] in a child for `case`, writing `stdin` and closing it (an
/// empty text writes nothing and leaves the pipe open until the child is
/// done), and asserts the child passed and said its claim.
fn child(case: &str, stdin: Option<&str>) {
    let mut running = gate_command(case)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = running.stdin.take().expect("a stdin pipe");
    let held = match stdin {
        Some("") => Some(pipe),
        Some(text) => {
            pipe.write_all(text.as_bytes()).unwrap();
            drop(pipe);
            None
        }
        None => {
            drop(pipe);
            None
        }
    };
    let output = running.wait_with_output().unwrap();
    drop(held);
    passed(case, &output);
}

/// Runs [`GATE`] in a child whose every thread spawned without a size of its
/// own is given [`UNMAPPABLE_STACK`] (the harness then runs the test on its
/// main thread), its stdin a file holding `go`: a file, because a child that
/// never reads may be gone before a pipe could be written. The child's gate
/// must release without reading it.
fn unspawnable_child() {
    let dir = crate::gdfl_fixtures::scratch("cov-start-unspawnable");
    let input = dir.join("stdin");
    std::fs::write(&input, b"go\n").unwrap();
    let output = gate_command("unspawnable")
        .env("RUST_MIN_STACK", UNMAPPABLE_STACK)
        .stdin(std::fs::File::open(&input).unwrap())
        .output()
        .unwrap();
    passed("unspawnable", &output);
    std::fs::remove_dir_all(dir).unwrap();
}

/// THE ANSWER IS `go` ONLY FOR `go` AND ITS LINE END, at the edges a pipe can
/// carry: `\r\n`, a lone `\r` with no newline (COMBINED: a carriage return
/// and the end of the input at once) and trailing blanks; never a leading
/// blank, another case, an empty line, nothing at all, a longer word or a
/// second word. (Only the first line is ever read: the child case
/// `upper-case-first`, COMBINED with a `go` second line.)
#[test]
fn only_go_and_its_line_end_is_go() {
    for (line, want) in [
        ("go\r\n", Gate::Go),
        ("go\r", Gate::Go),
        ("go \t\n", Gate::Go),
        (" go\n", Gate::Release),
        ("Go\n", Gate::Release),
        ("\n", Gate::Release),
        ("", Gate::Release),
        ("gone\n", Gate::Release),
        ("go now\n", Gate::Release),
    ] {
        assert_eq!(answer_of(Ok(Some(line.to_owned()))), want, "{line:?}");
    }
}

/// A fake gate that answers `go` at once.
struct Goes(Vec<(u64, ImportKind)>);

impl StartGate for Goes {
    fn claimed(&mut self, run: u64, kind: ImportKind) {
        self.0.push((run, kind));
    }

    fn wait(&mut self, _at_most: Duration) -> Gate {
        Gate::Go
    }
}

/// `await_go` with the production wait hands a claim heard `go` back, held,
/// after saying it was claimed.
#[test]
fn await_go_hands_back_a_claim_that_heard_go() {
    let store = crate::gdfl_fixtures::scratch("cov-start-go");
    let template = crate::gdfl_run::locks::Claim {
        run: 0,
        mode: "import".to_owned(),
        kind: "indices".to_owned(),
        batch: String::new(),
        source: "tickstore".to_owned(),
        from: "2024-04-01".to_owned(),
        to: "2024-04-01".to_owned(),
        filter: "*".to_owned(),
        verify: "off".to_owned(),
        restate: false,
        repair: "0".to_owned(),
        workers: 1,
        auto_resume: false,
        pid: 4242,
        host: "-".to_owned(),
        exe: "-".to_owned(),
        by: "api".to_owned(),
        started: 0,
        resumes: 0,
        resume_day: "-".to_owned(),
        legacy_ack: false,
        unstamped: false,
        request: "-".to_owned(),
    };
    let claimed = crate::gdfl_run::claim_with(&store, template, 0, None, 1_790_812_800).unwrap();
    let run = claimed.run;
    let mut gate = Goes(Vec::new());
    let held = await_go(claimed, &mut gate).unwrap();
    assert_eq!(gate.0, [(run, ImportKind::Indices)]);
    assert_eq!(held.run, run);
    held.release().unwrap();
}
