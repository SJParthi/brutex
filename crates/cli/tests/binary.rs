//! The binary itself, run once per command, so `main` is measured.
//!
//! `cargo test` replaces `main` with its own harness and never enters it, so
//! without this file the one call in `main.rs` is a line no test reaches — the
//! exact gap `crates/api/tests/binary.rs` exists to close for the other binary.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::process::Command;

/// The binary under test, as cargo built it beside this test.
fn bin() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_BIN_EXE_cli"));
    p.set_extension(std::env::consts::EXE_EXTENSION);
    p
}

/// One binary invocation with an already-existing store root.
///
/// Production must refuse a missing root before dispatch. These tests exercise
/// commands rather than that refusal, so they point the process at a root that
/// exists and give each process its own log directory.
///
/// # Why the root is per-process now
///
/// It was the bare `std::env::temp_dir()`, which every concurrent run of this
/// suite shares — and gate 23 clause C refuses a fixed temporary name for
/// exactly that reason. The log directory beside it was already per-process,
/// so the store was the odd one out rather than a deliberate exception.
///
/// Created rather than assumed: the whole point of naming it is that it is not
/// the one directory the operating system guarantees already exists.
fn command(tag: &str) -> Command {
    let root = std::env::temp_dir().join(format!(
        "brutex-cli-binary-store-{tag}-{}",
        std::process::id()
    ));
    let _made = std::fs::create_dir_all(&root);
    let mut command = Command::new(bin());
    command.env("BRUTEX_STORE", &root);
    command.env(
        "BRUTEX_LOG_DIR",
        std::env::temp_dir().join(format!(
            "brutex-cli-binary-log-{tag}-{}",
            std::process::id()
        )),
    );
    command
}

/// The number a report renders on a labelled row, as an integer.
///
/// The rows are `  <label><value right-aligned>  <note>`, so the value is the
/// FIRST whitespace-separated token AFTER the label. Not the last token on the
/// line: `swept` carries a percentage note, so a last-token parser reads
/// `0.0%` and fails to parse -- which is exactly what the first draft of this
/// helper did.
///
/// Returns `None` when the row is absent, which the caller asserts on rather
/// than defaulting past: a missing row means the report changed shape, and
/// silently treating that as zero would put this test back to passing on a
/// report it no longer understands.
fn row_number(text: &str, label: &str) -> Option<u64> {
    text.lines()
        .find(|l| l.trim_start().starts_with(label))?
        .trim_start()
        .strip_prefix(label)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[test]
fn a_sweep_runs_end_to_end_and_says_its_bars_were_generated() {
    // THE FIXTURE IS SIX SESSIONS, NOT TWO, AND THAT IS THE FIX.
    //
    // This test ran `sweep 2 50` and asserted `text.contains("bars")`. The
    // string "bars" is in the `BARS` section heading, which prints
    // unconditionally — so the assertion was satisfied by the report's own
    // furniture and could not fail.
    //
    // MEASURED, which is how this was found: the indicator warm-up is 1,876
    // bars, and a generated session is 375. So `sweep 2` offers 750 bars and
    // sweeps ZERO of them. The run this test called "end to end" produced
    // `swept 0`, `combinations found 0`, and a VERDICT of `NOTHING MEASURED`
    // with `trustworthy as a whole answer: NO`. The binary was telling the
    // truth; the test was not listening. `CLAUDE.md` section 4 bans a test that
    // asserts nothing, and one that passes on a run the binary itself declares
    // untrustworthy is that ban's exact case.
    //
    // Six sessions is the first count that clears the warm-up: 2,250 offered,
    // 374 swept. `min_hits` 200 keeps it at 8 ms and, importantly, leaves the
    // verdict `complete` rather than `REFUSED` — a budget breach would make the
    // "trustworthy" assertion below pin the wrong thing.
    let out = command("sweep")
        .args(["sweep", "6", "200"])
        .output()
        .expect("the binary runs");
    assert!(out.status.success(), "a valid sweep exits zero");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("THESE BARS ARE GENERATED"),
        "a report that does not declare its provenance is the one thing this \
         binary must never print:\n{text}"
    );

    // WORK HAPPENED. Each of these three is separately capable of failing on a
    // run that renders a complete-looking report having done nothing.
    let swept = row_number(&text, "swept").expect("the BARS block must report a swept count");
    assert!(
        swept > 0,
        "the sweep read no bars at all, so nothing below it means anything:\n{text}"
    );
    let combinations =
        row_number(&text, "combinations found").expect("the LADDER block must report a count");
    assert!(
        combinations > 0,
        "the ladder produced no combinations, so the report describes an empty \
         search:\n{text}"
    );

    // AND THE BINARY'S OWN VERDICT AGREES. This is the assertion the old test
    // most needed: the report already knew it had measured nothing and said so
    // in plain words, and nothing was reading them.
    assert!(
        text.contains("the frontier went extinct, which is the answer"),
        "the sweep must reach extinction on its own -- `NOTHING MEASURED` or \
         `REFUSED` here means this fixture stopped proving what it claims \
         to:\n{text}"
    );
    assert!(
        !text.contains("NOTHING MEASURED"),
        "the binary declared it measured nothing, which no end-to-end test may \
         pass on:\n{text}"
    );
}

#[test]
fn the_threshold_search_runs_end_to_end() {
    // SIX SESSIONS, AND THE SEARCH'S OWN NUMBERS ARE READ (P1-10-01). This ran
    // `auto 2` and checked only the provenance banner, which `auto` prints
    // whatever it found. MEASURED: two sessions are 750 bars against a 1,876
    // bar warm-up, so `auto 2` swept 0 bars, walked 0 ladders, chose threshold
    // `NONE`, said `NOTHING MEASURED` -- and the test passed. Six sessions is
    // the first count that clears the warm-up, as for the sweep test above.
    let out = command("auto")
        .args(["auto", "6"])
        .output()
        .expect("the binary runs");
    assert!(out.status.success(), "a valid auto exits zero");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("THESE BARS ARE GENERATED"),
        "provenance travels"
    );
    let swept = row_number(&text, "swept").expect("the BARS block reports swept");
    assert!(swept > 0, "the search read no bars:\n{text}");
    let threshold = row_number(&text, "threshold chosen")
        .expect("the SEARCH block names a numeric threshold, not NONE");
    assert!(threshold > 0, "no threshold was found:\n{text}");
    let ladders = row_number(&text, "ladders walked").expect("the SEARCH block counts ladders");
    assert!(ladders > 0, "no ladder was walked:\n{text}");
    let combinations =
        row_number(&text, "combinations found").expect("the LADDER block must report a count");
    assert!(combinations > 0, "the chosen ladder found nothing:\n{text}");
    assert!(
        text.contains("the frontier went extinct, which is the answer"),
        "the chosen rung must reach extinction:\n{text}"
    );
    assert!(
        !text.contains("NOTHING MEASURED") && !text.contains("REFUSED"),
        "the search declared it measured nothing or refused:\n{text}"
    );
}

#[test]
fn a_word_this_build_does_not_know_is_refused_by_name() {
    let out = command("unknown")
        .args(["backtest"])
        .output()
        .expect("the binary runs");
    assert_eq!(
        out.status.code(),
        Some(i32::from(cli::MISUSED)),
        "an unknown command is a misuse, not a failure"
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("backtest"),
        "the refusal names the word it objected to:\n{text}"
    );
    assert!(text.contains("usage:"), "and prints the usage:\n{text}");
}

#[test]
fn no_command_at_all_is_refused_rather_than_defaulted() {
    let out = command("empty").output().expect("the binary runs");
    assert_eq!(out.status.code(), Some(i32::from(cli::MISUSED)));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("no command given"),
        "silence is not a command, and guessing one would be the fallback \
        CLAUDE.md section 4 bans:\n{text}"
    );
}

#[test]
fn a_missing_store_refuses_before_logging_or_dispatch() {
    let root = std::env::temp_dir().join(format!(
        "brutex-cli-binary-missing-store-{}",
        std::process::id()
    ));
    let logs = std::env::temp_dir().join(format!(
        "brutex-cli-binary-missing-store-logs-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&root);
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&logs);

    let out = Command::new(bin())
        .env("BRUTEX_STORE", &root)
        .env("BRUTEX_LOG_DIR", &logs)
        .args(["sweep", "6", "200"])
        .output()
        .expect("the binary runs to its preflight");
    assert_eq!(
        out.status.code(),
        Some(i32::from(cli::FAILED)),
        "an unavailable configured store is an operational failure"
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.starts_with("refused: ") && text.contains(&root.display().to_string()),
        "the refusal names the exact unavailable root:\n{text}"
    );
    assert!(
        !text.contains("events ->") && !text.contains("THESE BARS ARE GENERATED"),
        "neither logging nor dispatch may precede the store preflight:\n{text}"
    );
    assert!(!root.exists(), "the missing store was not created");
    assert!(!logs.exists(), "the log sink was not installed");
}

/// **An argument that is not UTF-8 is refused as a misuse, not a panic.**
/// probeapi-3, D-0998.
///
/// `main` collected `std::env::args()`, which panics on the first argument
/// that is not valid Unicode: exit 101, a Rust backtrace hint on stderr, and
/// none of the normal exit lines. It now reads `args_os`, refuses by name with
/// `MISUSED`, prints where its events went as every run does, and records the
/// refusal in the log.
#[cfg(unix)]
#[test]
fn a_non_utf8_argument_is_refused_by_name_not_a_panic() {
    use std::os::unix::ffi::OsStrExt;
    let logs = std::env::temp_dir().join(format!(
        "brutex-cli-binary-log-nonutf8-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&logs);
    let out = command("nonutf8")
        .arg("sweep")
        .arg(std::ffi::OsStr::from_bytes(b"6\xff"))
        .arg("200")
        .output()
        .expect("the binary runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(i32::from(cli::MISUSED)),
        "a word that is not text is a misuse:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(
        stdout.contains("refused: argument 2 is not valid UTF-8"),
        "the refusal names which argument:\n{stdout}"
    );
    assert!(stdout.contains("usage:"), "and prints the usage:\n{stdout}");
    assert!(
        stdout.contains("events ->"),
        "the normal exit line still says where events went:\n{stdout}"
    );
    assert!(
        !stdout.contains("THESE BARS ARE GENERATED"),
        "nothing was dispatched:\n{stdout}"
    );
    let logged = std::fs::read_dir(&logs)
        .expect("the log directory exists")
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect::<String>();
    assert!(
        logged.contains("not valid UTF-8"),
        "the refusal is in the log:\n{logged}"
    );
}

/// **A closed stdout is said on stderr, never a panic.** v53-2, D-1484.
///
/// `cli sweep 6 100 | true` exited 101 with a panic backtrace. The pipe's
/// read end is closed BEFORE the child starts, so the first write fails with
/// EPIPE every time rather than when a race happens to lose.
#[test]
fn a_closed_stdout_is_said_on_stderr_and_never_panics() {
    let (reader, writer) = std::io::pipe().expect("a pipe");
    drop(reader);
    let logs = std::env::temp_dir().join(format!(
        "brutex-cli-binary-log-closed-stdout-{}",
        std::process::id()
    ));
    let _stale = std::fs::remove_dir_all(&logs);
    let out = command("closed-stdout")
        .args(["sweep", "6", "200"])
        .stdout(writer)
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("the binary runs");
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(i32::from(cli::OK)),
        "a reader that closed the pipe leaves the run's own code:\n{said}"
    );
    assert!(!said.contains("panicked"), "{said}");
    assert!(said.contains("stdout is not writable"), "{said}");
    assert!(said.contains("closed the pipe"), "{said}");

    // AND THE FAILURE IS LOGGED, ONCE (P1-17-03). C-V53-02 says each failure
    // emits one `cli.output` event, and nothing read the log: deleting the
    // `note(..)` in `cli::deliver` left this green. The run's own log
    // directory is read back.
    let events = output_events(&logs);
    assert_eq!(
        events.len(),
        1,
        "one `cli.output` event for the one failed write:\n{events:#?}\n{said}"
    );
    let event = events.first().map_or("", String::as_str);
    assert!(
        event.contains("\"level\":\"warn\"") && event.contains("\"exit\":0"),
        "a WARN that records the exit code the run kept: {event}"
    );
    let _cleaned = std::fs::remove_dir_all(&logs);
}

/// Every `cli.output` line written under `dir`, at any depth.
fn output_events(dir: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(at) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(text) = std::fs::read_to_string(&path) {
                found.extend(
                    text.lines()
                        .filter(|line| line.contains("\"target\":\"cli.output\""))
                        .map(str::to_owned),
                );
            }
        }
    }
    found
}
