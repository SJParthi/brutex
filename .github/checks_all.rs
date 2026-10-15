//! One command for every check, and a dated result naming what is red
//! (R3-14; NEW-02, RA-184).
//!
//! # What was missing
//!
//! The definition of done (CLAUDE.md section 9) is a list of gates, and every
//! package ran its own subset by hand: `cargo fmt`, the clippy and tests of
//! the crates it touched, now and then a `.github` tool. Nothing ran all of
//! them on one tree and wrote down which were red, so "the gates are green"
//! was a sentence, not a file, and a red gate on the merged tree (gate 10's
//! rows naming tests that do not exist, a test that fails only on macOS) was
//! found by an audit days later.
//!
//! # Why this tool starts no program
//!
//! Gate 0 refuses a `.github/*.rs` that starts any program but `git`, `mkfifo`
//! and the browser openers (D-2344), and a runner of `cargo` would be exactly
//! that. So the tool is split where the law splits it: THIS FILE is the list
//! of checks and the verdict over their results, and the loop that runs the
//! list is run by a shell. The loop's TEXT is carried here ([`RUNNER`],
//! printed by `checks_all script`), so a fresh clone holds the one command:
//! `checks_all script | bash -s -- <dir>` from the repository root. Widening
//! D-2344 so a tracked tool may start `cargo` is a decision this package does
//! not take on its own.
//!
//! ```text
//! checks_all plan [--skip-test NAME]...  > plan.tsv
//!     one check a line: <name> TAB <what it proves> TAB <command>, in the
//!     order to run them; the commands are run from the repository root by
//!     `bash -c` with CHECKS_BIN naming a scratch directory for built tools,
//!     and each one's output goes to <dir>/<name>.log and its exit status to
//!     <dir>/<name>.exit (and its seconds to <dir>/<name>.secs).
//! checks_all step <job> <n>
//!     prints the script of the n-th `run:` step (from 0) of <job> in
//!     .github/workflows/ci.yml, which a plan line runs as the runner would.
//! checks_all script
//!     prints the loop that runs the plan ([`RUNNER`]), for `bash -s -- <dir>`:
//!     the one command, carried in the repository.
//! checks_all verdict <dir>
//!     reads <dir>/plan.tsv and every check's .exit and .log, writes
//!     <dir>/checks-<UTC>.md naming each check GREEN, RED or NOT RUN (with the
//!     failing tests' names and the log's last lines), and exits 1 when any
//!     check is red or did not run.
//! ```
//!
//! A test skipped by name (`--skip-test`) is written into the plan's own
//! command and repeated in the result's header, so a result never reads
//! greener than what ran.
//!
//! # Every gate, read from the workflow itself
//!
//! The gates are the `run:` steps of `.github/workflows/ci.yml`, and a list
//! of them kept here would rot the day a gate is added. So the plan READS the
//! workflow ([`ci_steps`]): every step with a `run:` block, in job order,
//! becomes one check, `ci-<job>-<step>`, whose command asks this tool for
//! the step's script (`checks_all step <job> <n>`) and runs it the way the
//! runner does (`bash --noprofile --norc -eo pipefail`), with a `RUNNER_TEMP`
//! and a `GITHUB_ENV` file of its JOB under `CHECKS_BIN`, which every later
//! step of that job reads and no other job sees, as on a runner, and a
//! `TMPDIR` of its job there too, for every tool that honours it (macOS's
//! `mktemp -d` does not: measured on 14 October 2026, it writes under the
//! user's own temporary directory whatever `TMPDIR` says, so on a Mac the run
//! needs that directory writable). Three kinds of step are not run again,
//! each said in the result: the workspace's `cargo fmt`, `cargo clippy` and
//! `cargo test`
//! steps, which the plan's own `fmt`, `clippy` and `test` run (honouring
//! `--skip-test`); the mutation jobs ([`CI_ONLY_JOBS`]), hours of work a
//! runner farm does; and a step that would change the machine rather than
//! the worktree ([`CHANGES_THE_MACHINE`]: a `cargo install`, an `npm ci`).
//! The last two exit 125 and are written NOT RUN HERE, never green. A step the Mac cannot run (a GNU-only flag, a tool not installed)
//! is RED with its log's tail, which is the point: the result names it.

#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

/// One check: its name, what it proves, and its command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// A file-name-safe name: `fmt`, `clippy`, `tool-gates_routes`.
    pub name: String,
    /// What a green run of it proves.
    pub proves: String,
    /// The `bash -c` command, run from the repository root.
    pub command: String,
}

/// THE ONE COMMAND, carried in the repository so a fresh clone has it: the
/// shell loop that runs the plan, printed by `checks_all script` and piped to
/// `bash`. This tool starts no program (gate 0, D-2344); the shell that reads
/// this text does. From the repository root:
///
/// ```text
/// rustc --edition=2024 .github/checks_all.rs -o "$TMPDIR/checks_all" &&
///   "$TMPDIR/checks_all" script | bash -s -- <out dir> [--skip-test NAME]...
/// ```
///
/// It builds this tool again into `<out dir>/bin`, writes `plan.tsv`, runs
/// every check from the root with its log, exit status and seconds beside it
/// (and one line a check in `progress.txt`), then writes the dated result
/// `checks-<UTC>.md` and exits with the verdict's status. It deletes nothing.
/// `CHECKS_BOARD` and `CHECKS_RULES` name the board for the `owners` check,
/// which is NOT RUN HERE without them.
pub const RUNNER: &str = r#"set -u
out=${1:?usage: checks_all script | bash -s -- <out dir> [--skip-test NAME]...}
shift
mkdir -p "$out/bin" || exit 2
export CHECKS_BIN="$out/bin"
export CHECKS_RUN_ID="${CHECKS_RUN_ID:-$(date -u +%s)}"
rustc --edition=2024 -D warnings .github/checks_all.rs -o "$out/bin/checks_all" || exit 2
"$out/bin/checks_all" plan "$@" > "$out/plan.tsv" || exit 2
while IFS=$'\t' read -r name proves cmd; do
  start=$(date +%s)
  echo "$(date -u +%H:%M:%S) start $name" >> "$out/progress.txt"
  bash -c "$cmd" > "$out/$name.log" 2>&1 < /dev/null
  echo $? > "$out/$name.exit"
  echo $(( $(date +%s) - start )) > "$out/$name.secs"
  echo "$(date -u +%H:%M:%S) end $name exit $(cat "$out/$name.exit")" >> "$out/progress.txt"
done < "$out/plan.tsv"
"$out/bin/checks_all" verdict "$out"
"#;

/// The `.github/*.rs` tools under `root`, sorted.
///
/// # Errors
///
/// The directory cannot be read, or holds no tool: a blind plan.
pub fn tools(root: &Path) -> Result<Vec<String>, String> {
    let dir = root.join(".github");
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|why| format!("{}: {why}", dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|n| n.strip_suffix(".rs").map(str::to_owned))
        .collect();
    out.sort();
    if out.is_empty() {
        return Err(format!("{} holds no tool", dir.display()));
    }
    Ok(out)
}

/// The workflow the gates are read from, relative to the root.
pub const WORKFLOW: &str = ".github/workflows/ci.yml";

/// Jobs whose steps run only on CI's runners, each with why. Written NOT RUN
/// HERE, never green.
pub const CI_ONLY_JOBS: [(&str, &str); 3] = [
    (
        "mutant-plan",
        "it plans mutation testing, which takes hours a check run on one machine does not have",
    ),
    (
        "mutants",
        "mutation testing takes hours a check run on one machine does not have",
    ),
    (
        "ci-ok",
        "it reads the results of the CI run's other jobs, which only CI's runner has",
    ),
];

/// Words a step's script may not hold to be run on the operator's machine,
/// each with why: a check run changes the worktree it checks and nothing
/// else. Such a step is written NOT RUN HERE, never green.
pub const CHANGES_THE_MACHINE: [(&str, &str); 2] = [
    (
        "cargo install",
        "installs a tool into the user's own toolchain; the installed one is used by the plan's \
         own checks",
    ),
    (
        "npm ci",
        "deletes and refills web/node_modules, which in a worktree here is a link to another \
         checkout's",
    ),
];

/// The status a check exits with to say it was not run here, by design.
pub const NOT_HERE: i32 = 125;

/// `text` as ONE shell word that runs nothing: single-quoted, each `'` closed,
/// escaped and reopened. A tab or a line break becomes a space first, because
/// the plan is one line a check with tab-separated fields.
///
/// R3-14 repair 1: a declined step's reason was pasted inside `echo '...'`,
/// and a reason holding an apostrophe ("another checkout's") ended the quote
/// early. The `npm ci` line was then a syntax error (exit 2, reported RED
/// instead of NOT RUN HERE) and the `cargo install` line ran the rest of its
/// sentence as a command.
#[must_use]
pub fn shell_word(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| {
            if matches!(c, '\t' | '\n' | '\r') {
                ' '
            } else {
                c
            }
        })
        .collect();
    format!("'{}'", flat.replace('\'', r"'\''"))
}

/// The command of a check declined on this machine: prints `why` and exits
/// [`NOT_HERE`], whatever `why` says.
#[must_use]
pub fn declined_command(why: &str) -> String {
    format!(r"printf '%s\n' {}; exit {NOT_HERE}", shell_word(why))
}

/// One `run:` step of the workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiStep {
    /// The job it belongs to.
    pub job: String,
    /// Its `name:`, or its first script line when it has none.
    pub name: String,
    /// Its script, de-indented.
    pub script: String,
}

/// Leading spaces of `line`.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Every step with a `run:` of the workflow `yml`, in order: a two-space
/// key under `jobs:` opens a job, a `- name:` or `- uses:` at any depth opens
/// a step, and `run: |` takes every following line indented deeper than the
/// key (blank lines kept), de-indented by the first line's depth.
///
/// # Errors
///
/// No `jobs:` key, or no step with a script: a workflow this reader does not
/// understand, said rather than read as "no gates".
pub fn ci_steps(yml: &str) -> Result<Vec<CiStep>, String> {
    let lines: Vec<&str> = yml.lines().collect();
    let Some(start) = lines.iter().position(|l| *l == "jobs:") else {
        return Err(format!("{WORKFLOW} has no `jobs:` key"));
    };
    let mut out = Vec::new();
    let mut job = String::new();
    let mut name: Option<String> = None;
    let mut i = start + 1;
    while let Some(line) = lines.get(i) {
        let depth = indent(line);
        let body = line.trim_start();
        i += 1;
        if depth == 2 && body.ends_with(':') && !body.starts_with('#') {
            job = body.trim_end_matches(':').to_owned();
            name = None;
            continue;
        }
        if let Some(rest) = body.strip_prefix("- ") {
            name = rest.strip_prefix("name:").map(|n| n.trim().to_owned());
        }
        let key = body.strip_prefix("- ").unwrap_or(body);
        let Some(run) = key.strip_prefix("run:") else {
            continue;
        };
        let run = run.trim();
        let script = if run == "|" || run == "|-" {
            let mut block: Vec<&str> = Vec::new();
            while let Some(next) = lines.get(i) {
                if !next.trim().is_empty() && indent(next) <= depth {
                    break;
                }
                block.push(next);
                i += 1;
            }
            let cut = block
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| indent(l))
                .min()
                .unwrap_or(0);
            block
                .iter()
                .map(|l| l.get(cut..).unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n")
                .trim_end()
                .to_owned()
        } else {
            run.to_owned()
        };
        let named = name
            .clone()
            .unwrap_or_else(|| script.lines().next().unwrap_or("").to_owned());
        out.push(CiStep {
            job: job.clone(),
            name: named,
            script: format!("{script}\n"),
        });
    }
    if out.is_empty() {
        return Err(format!("{WORKFLOW} has no step with a `run:`"));
    }
    Ok(out)
}

/// A file-name-safe slug of `text`, at most 48 bytes.
#[must_use]
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= 48 {
            break;
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// Whether `step` is one the plan's own workspace checks already run.
fn run_by_the_plan(step: &CiStep) -> bool {
    let script = step.script.trim();
    script == "cargo fmt --all --check"
        || script.starts_with("cargo clippy --workspace")
        || script == "cargo test --workspace --locked"
}

/// The checks the workflow's steps become, in order (see the module's
/// "Every gate" section).
#[must_use]
pub fn ci_checks(steps: &[CiStep]) -> Vec<Check> {
    let mut out = Vec::new();
    let mut per_job: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for step in steps {
        let n = per_job.entry(step.job.as_str()).or_insert(0);
        let index = *n;
        *n += 1;
        if run_by_the_plan(step) {
            continue;
        }
        let name = format!("ci-{}-{}", slug(&step.job), slug(&step.name));
        let name = if out.iter().any(|c: &Check| c.name == name) {
            format!("{name}-{index}")
        } else {
            name
        };
        let proves = format!("{} ({}, CI)", step.name.replace('\t', " "), step.job);
        let declined = if let Some((_, why)) = CI_ONLY_JOBS.iter().find(|(job, _)| *job == step.job)
        {
            Some(format!("the {} job runs on CI only: {why}", step.job))
        } else {
            CHANGES_THE_MACHINE
                .iter()
                .find(|(word, _)| step.script.contains(word))
                .map(|(word, why)| format!("it runs `{word}`, which {why}"))
        };
        let command = if let Some(why) = declined {
            declined_command(&format!("not run here: {why}"))
        } else {
            // THE RUNNER'S FILES AND IDS, EMULATED (R3-14 repair 1): a step
            // that writes `$GITHUB_OUTPUT` or names `$GITHUB_RUN_ID` and
            // `$GITHUB_RUN_ATTEMPT` failed here on the unset variable, and
            // the result called that the tree's RED.
            let job = slug(&step.job);
            format!(
                "mkdir -p \"$CHECKS_BIN/runner-temp-{job}\" \"$CHECKS_BIN/tmp-{job}\" && touch \
                 \"$CHECKS_BIN/github-{job}.env\" \"$CHECKS_BIN/github-{job}.output\" && \
                 \"$CHECKS_BIN/checks_all\" step {} {index} > \"$CHECKS_BIN/{name}.sh\" && \
                 (set -a; . \"$CHECKS_BIN/github-{job}.env\"; set +a; CI=true GITHUB_WORKSPACE=\"$PWD\" \
                 RUNNER_TEMP=\"$CHECKS_BIN/runner-temp-{job}\" TMPDIR=\"$CHECKS_BIN/tmp-{job}\" \
                 GITHUB_ENV=\"$CHECKS_BIN/github-{job}.env\" \
                 GITHUB_OUTPUT=\"$CHECKS_BIN/github-{job}.output\" \
                 GITHUB_RUN_ID=\"$CHECKS_RUN_ID\" GITHUB_RUN_ATTEMPT=1 \
                 bash --noprofile --norc -eo pipefail \"$CHECKS_BIN/{name}.sh\")",
                step.job
            )
        };
        out.push(Check {
            name,
            proves,
            command,
        });
    }
    out
}

/// Every check, in the order to run them: the workspace gates of CLAUDE.md
/// section 9, then each `.github` tool's own tests and gate 6c's format and
/// clippy, then the checks that read the tree, then every workflow step
/// ([`ci_checks`]).
#[must_use]
pub fn plan(tools: &[String], steps: &[CiStep], skip: &[String]) -> Vec<Check> {
    let check = |name: &str, proves: &str, command: String| Check {
        name: name.to_owned(),
        proves: proves.to_owned(),
        command,
    };
    let skips: String = skip.iter().map(|s| format!(" --skip {s}")).collect();
    let test_args = if skips.is_empty() {
        String::new()
    } else {
        format!(" --{skips}")
    };
    let mut out = vec![
        check(
            "fmt",
            "every crate is formatted (gate 6a)",
            "cargo fmt --all --check".to_owned(),
        ),
        check(
            "clippy",
            "every target of every crate is clippy-clean under -D warnings (gate 6b)",
            "cargo clippy --workspace --all-targets --locked -- -D warnings".to_owned(),
        ),
        // `--no-fail-fast`: without it cargo stops at the first crate whose
        // tests fail, and every crate after it goes unrun and unnamed, so the
        // result would read greener than the tree (measured on 14 October
        // 2026: one `pull` failure hid every crate after `pull`).
        check(
            "test",
            "every test of every crate passes (CLAUDE.md section 9)",
            format!("cargo test --workspace --locked --no-fail-fast{test_args}"),
        ),
        check(
            "deny",
            "the dependency policy holds (gate 3)",
            "cargo deny check".to_owned(),
        ),
        check(
            "tools-fmt",
            "every .github tool is formatted (gate 6c)",
            format!(
                "rustfmt --edition 2024 --check {}",
                tools
                    .iter()
                    .map(|t| format!(".github/{t}.rs"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ),
    ];
    for tool in tools {
        out.push(check(
            &format!("tool-{tool}"),
            "the tool's own tests pass, and it and its tests are clippy-clean (gates 0, 6c)",
            format!(
                "rustc --edition=2024 --test -D warnings .github/{tool}.rs -o \
                 \"$CHECKS_BIN/{tool}-tests\" && \"$CHECKS_BIN/{tool}-tests\" && \
                 clippy-driver --edition=2024 -D warnings .github/{tool}.rs -o \
                 \"$CHECKS_BIN/{tool}-clippy\" && clippy-driver --edition=2024 --test -D \
                 warnings .github/{tool}.rs -o \"$CHECKS_BIN/{tool}-clippy-t\""
            ),
        ));
    }
    out.push(check(
        "routes",
        "every served route has a limits row and a growth bench, or a line naming who owes it",
        "rustc --edition=2024 -D warnings .github/gates_routes.rs -o \"$CHECKS_BIN/gates_routes\" \
         && \"$CHECKS_BIN/gates_routes\" ."
            .to_owned(),
    ));
    out.push(check(
        "owners",
        "every confirmed finding and every audit row has an owner (needs CHECKS_BOARD and \
         CHECKS_RULES: the board is outside the repository)",
        format!(
            "if [ -z \"${{CHECKS_BOARD:-}}\" ] || [ -z \"${{CHECKS_RULES:-}}\" ]; then {}; fi; \
             rustc --edition=2024 -D warnings \
             .github/findings_owner.rs -o \"$CHECKS_BIN/findings_owner\" && \
             \"$CHECKS_BIN/findings_owner\" \"$CHECKS_BOARD/audit-result.json\" \
             \"$CHECKS_BOARD/fix-plan.json\" \"$CHECKS_BOARD/plan-round3-20261012.json\" \
             \"$CHECKS_RULES\"",
            declined_command(
                "not run here: CHECKS_BOARD and CHECKS_RULES name the board, which lives \
                 outside the repository"
            )
        ),
    ));
    out.extend(ci_checks(steps));
    out
}

/// The plan as the shell reads it: one check a line, three fields.
#[must_use]
pub fn plan_tsv(checks: &[Check]) -> String {
    checks
        .iter()
        .map(|c| format!("{}\t{}\t{}\n", c.name, c.proves, c.command))
        .collect()
}

/// Reads a plan back. A line without three fields is refused.
///
/// # Errors
///
/// The line that is not a check.
pub fn read_plan(tsv: &str) -> Result<Vec<Check>, String> {
    let mut out = Vec::new();
    for (n, line) in tsv.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let mut parts = line.splitn(3, '\t');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(name), Some(proves), Some(command))
                if !name.is_empty()
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') =>
            {
                out.push(Check {
                    name: name.to_owned(),
                    proves: proves.to_owned(),
                    command: command.to_owned(),
                });
            }
            _ => {
                return Err(format!(
                    "plan line {} is not name TAB proves TAB command",
                    n + 1
                ));
            }
        }
    }
    if out.is_empty() {
        return Err("the plan holds no check".to_owned());
    }
    Ok(out)
}

/// What became of one check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Exit status 0.
    Green,
    /// Any other status, with the failing tests the log names.
    Red(i32, Vec<String>),
    /// No status was written: it did not run, or did not finish.
    NotRun,
    /// It exited [`NOT_HERE`]: by design not run on this machine (a CI-only
    /// job). Never counted green, never counted red.
    NotHere,
}

/// The failing test names a cargo or libtest log reports.
#[must_use]
pub fn failing_tests(log: &str) -> Vec<String> {
    let mut out: Vec<String> = log
        .lines()
        .filter_map(|l| {
            l.strip_prefix("test ")
                .and_then(|r| r.strip_suffix(" ... FAILED"))
                .map(str::to_owned)
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The state of one check from its `.exit` text and its log.
#[must_use]
pub fn state_of(exit: Option<&str>, log: &str) -> State {
    match exit.map(str::trim) {
        None | Some("") => State::NotRun,
        Some("0") => State::Green,
        Some(code) if code.parse() == Ok(NOT_HERE) => State::NotHere,
        Some(code) => State::Red(code.parse().unwrap_or(-1), failing_tests(log)),
    }
}

/// `secs` since the epoch as `YYYY-MM-DDTHH-MM-SSZ` (UTC), file-name safe.
#[must_use]
pub fn utc_stamp(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Days to a civil date (proleptic Gregorian), H. Hinnant's algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}-{:02}-{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

/// The result file's text for `checks`, each with its state, seconds and
/// log tail.
#[must_use]
pub fn result(stamp: &str, rows: &[(Check, State, Option<u64>, String)]) -> String {
    let red = rows
        .iter()
        .filter(|r| matches!(r.1, State::Red(..)))
        .count();
    let not_run = rows.iter().filter(|r| r.1 == State::NotRun).count();
    let not_here = rows.iter().filter(|r| r.1 == State::NotHere).count();
    let mut out = format!("# Checks, {stamp}\n\n");
    let _ = writeln!(
        out,
        "{} checks: {} green, {red} red, {not_run} not run, {not_here} not run here (CI only). {}\n",
        rows.len(),
        rows.len() - red - not_run - not_here,
        match (red + not_run, not_here) {
            (0, 0) => "EVERY CHECK IS GREEN.",
            (0, _) =>
                "EVERY CHECK RUN HERE IS GREEN; the rows marked NOT RUN HERE are proved by CI only.",
            _ => "NOT GREEN: see the rows marked RED and NOT RUN.",
        }
    );
    let skipped: Vec<&str> = rows
        .iter()
        .flat_map(|r| r.0.command.split(" --skip ").skip(1))
        .map(|s| s.split_whitespace().next().unwrap_or(""))
        .collect();
    if !skipped.is_empty() {
        let _ = writeln!(
            out,
            "Skipped by name, so not proved by this run: {}\n",
            skipped.join(", ")
        );
    }
    out.push_str("| check | state | seconds | proves |\n|---|---|---|---|\n");
    for (check, state, secs, _) in rows {
        let state = match state {
            State::Green => "GREEN".to_owned(),
            State::Red(code, _) => format!("RED (exit {code})"),
            State::NotRun => "NOT RUN".to_owned(),
            State::NotHere => "NOT RUN HERE (CI only)".to_owned(),
        };
        let _ = writeln!(
            out,
            "| {} | {state} | {} | {} |",
            check.name,
            secs.map_or_else(|| "-".to_owned(), |s| s.to_string()),
            check.proves
        );
    }
    for (check, state, _, tail) in rows {
        if matches!(state, State::Green | State::NotHere) {
            continue;
        }
        let _ = writeln!(out, "\n## {}\n\n`{}`\n", check.name, check.command);
        if let State::Red(_, failing) = state
            && !failing.is_empty()
        {
            let _ = writeln!(out, "Failing tests: {}\n", failing.join(", "));
        }
        if !tail.is_empty() {
            let _ = writeln!(out, "```text\n{tail}\n```");
        }
    }
    out
}

/// The last `n` lines of `log`.
#[must_use]
pub fn tail(log: &str, n: usize) -> String {
    let lines: Vec<&str> = log.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

fn verdict(dir: &Path) -> Result<(String, bool), String> {
    let read =
        |p: &Path| std::fs::read_to_string(p).map_err(|why| format!("{}: {why}", p.display()));
    let checks = read_plan(&read(&dir.join("plan.tsv"))?)?;
    let mut rows = Vec::new();
    for check in checks {
        let exit = std::fs::read_to_string(dir.join(format!("{}.exit", check.name))).ok();
        let log =
            std::fs::read_to_string(dir.join(format!("{}.log", check.name))).unwrap_or_default();
        let secs = std::fs::read_to_string(dir.join(format!("{}.secs", check.name)))
            .ok()
            .and_then(|s| s.trim().parse().ok());
        let state = state_of(exit.as_deref(), &log);
        let tail = if matches!(state, State::Green | State::NotHere) {
            String::new()
        } else {
            tail(&log, 15)
        };
        rows.push((check, state, secs, tail));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let stamp = utc_stamp(now);
    let text = result(&stamp, &rows);
    let path = dir.join(format!("checks-{stamp}.md"));
    std::fs::write(&path, &text).map_err(|why| format!("{}: {why}", path.display()))?;
    let green = rows
        .iter()
        .all(|r| matches!(r.1, State::Green | State::NotHere));
    Ok((format!("{}\n{text}", path.display()), green))
}

#[allow(
    dead_code,
    reason = "the entry point when built by rustc; unused under --test"
)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("plan") => {
            let mut skip = Vec::new();
            let mut rest = args.iter().skip(1);
            while let Some(arg) = rest.next() {
                match (arg.as_str(), rest.next()) {
                    ("--skip-test", Some(name)) => skip.push(name.clone()),
                    _ => {
                        eprintln!("checks_all plan takes only --skip-test NAME pairs");
                        return ExitCode::from(2);
                    }
                }
            }
            let steps = std::fs::read_to_string(WORKFLOW)
                .map_err(|why| format!("{WORKFLOW}: {why}"))
                .and_then(|yml| ci_steps(&yml));
            match (tools(Path::new(".")), steps) {
                (Ok(tools), Ok(steps)) => {
                    print!("{}", plan_tsv(&plan(&tools, &steps, &skip)));
                    ExitCode::SUCCESS
                }
                (Err(why), _) | (_, Err(why)) => {
                    eprintln!("checks_all refused: {why}");
                    ExitCode::from(2)
                }
            }
        }
        Some("step") if args.len() == 3 => {
            let index: Option<usize> = args[2].parse().ok();
            let steps = std::fs::read_to_string(WORKFLOW)
                .map_err(|why| format!("{WORKFLOW}: {why}"))
                .and_then(|yml| ci_steps(&yml));
            match steps {
                Ok(steps) => {
                    let found = steps
                        .iter()
                        .filter(|s| s.job == args[1])
                        .nth(index.unwrap_or(usize::MAX));
                    if let Some(step) = found {
                        print!("{}", step.script);
                        ExitCode::SUCCESS
                    } else {
                        eprintln!(
                            "checks_all: {WORKFLOW} has no step {} in job {}",
                            args[2], args[1]
                        );
                        ExitCode::from(2)
                    }
                }
                Err(why) => {
                    eprintln!("checks_all refused: {why}");
                    ExitCode::from(2)
                }
            }
        }
        Some("verdict") if args.len() == 2 => match verdict(Path::new(&args[1])) {
            Ok((text, green)) => {
                print!("{text}");
                if green {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(why) => {
                eprintln!("checks_all refused: {why}");
                ExitCode::from(2)
            }
        },
        Some("script") if args.len() == 1 => {
            print!("{RUNNER}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: checks_all plan [--skip-test NAME]... | checks_all step <job> <n> | \
                 checks_all verdict <dir> | checks_all script"
            );
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools_here() -> Vec<String> {
        vec!["gates_routes".to_owned(), "findings_owner".to_owned()]
    }

    #[test]
    fn the_plan_runs_the_workspace_gates_then_every_tool_and_reads_back() {
        let checks = plan(&tools_here(), &[], &[]);
        let names: Vec<&str> = checks.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "fmt",
                "clippy",
                "test",
                "deny",
                "tools-fmt",
                "tool-gates_routes",
                "tool-findings_owner",
                "routes",
                "owners"
            ]
        );
        assert_eq!(
            checks[2].command, "cargo test --workspace --locked --no-fail-fast",
            "nothing skipped unless asked"
        );
        assert!(
            checks[4]
                .command
                .ends_with(".github/gates_routes.rs .github/findings_owner.rs")
        );
        let back = read_plan(&plan_tsv(&checks)).expect("the plan reads back");
        assert_eq!(back, checks);
        assert!(read_plan("").is_err());
        assert!(read_plan("bad name\tx\ty\n").is_err());
        assert!(read_plan("fmt\tonly two\n").is_err());
    }

    /// THE ONE COMMAND IS CARRIED IN THE REPOSITORY: the loop `checks_all
    /// script` prints builds this tool, writes the plan it runs, runs every
    /// line with its log, exit status and seconds beside it, then writes the
    /// verdict; it forwards `--skip-test` to the plan, deletes nothing, and the
    /// `owners` check declines (NOT RUN HERE) when no board is named rather
    /// than reading as red or as green.
    #[test]
    fn the_one_command_is_carried_here_and_the_owners_check_declines_without_a_board() {
        let at = |needle: &str| {
            RUNNER
                .find(needle)
                .unwrap_or_else(|| panic!("the runner lacks {needle:?}:\n{RUNNER}"))
        };
        let built = at("rustc --edition=2024 -D warnings .github/checks_all.rs");
        let planned = at("checks_all\" plan \"$@\" > \"$out/plan.tsv\"");
        let looped = at("done < \"$out/plan.tsv\"");
        let judged = at("checks_all\" verdict \"$out\"");
        assert!(built < planned && planned < looped && looped < judged);
        for kept in ["$name.log", "$name.exit", "$name.secs", "progress.txt"] {
            let _found = at(kept);
        }
        assert!(at("IFS=$'\\t'") < looped, "the plan's fields are tab-split");
        for removal in ["rm ", "rmdir", "unlink", "-delete", "git clean"] {
            assert!(!RUNNER.contains(removal), "the runner deletes nothing");
        }
        let owners = plan(&tools_here(), &[], &[])
            .into_iter()
            .find(|c| c.name == "owners")
            .expect("the owners check");
        let guard = owners
            .command
            .find(&format!("exit {NOT_HERE}; fi;"))
            .expect("the owners check declines without a board");
        let ran = owners
            .command
            .find("findings_owner\" \"$CHECKS_BOARD/audit-result.json\"")
            .expect("the owners check reads the board");
        assert!(guard < ran, "{}", owners.command);
        assert!(owners.command.contains("${CHECKS_BOARD:-}"));
        assert!(owners.command.contains("${CHECKS_RULES:-}"));
    }

    #[test]
    fn a_skipped_test_is_in_the_command_and_in_the_result() {
        let checks = plan(
            &tools_here(),
            &[],
            &["slow_one".to_owned(), "other".to_owned()],
        );
        assert_eq!(
            checks[2].command,
            "cargo test --workspace --locked --no-fail-fast -- --skip slow_one --skip other"
        );
        let rows: Vec<_> = checks
            .into_iter()
            .map(|c| (c, State::Green, Some(1), String::new()))
            .collect();
        let text = result("2026-10-13T00-00-00Z", &rows);
        assert!(
            text.contains("Skipped by name, so not proved by this run: slow_one, other"),
            "{text}"
        );
        assert!(text.contains("EVERY CHECK IS GREEN."), "{text}");
    }

    /// A red or missing check is named, with its failing tests and its log's
    /// tail; a green result is only written when every check ran green.
    #[test]
    fn the_result_names_what_is_red_and_what_did_not_run() {
        let log = "running 3 tests\ntest a::one ... ok\ntest a::two ... FAILED\ntest b::three ... FAILED\nfailures:\n";
        assert_eq!(failing_tests(log), ["a::two", "b::three"]);
        assert_eq!(state_of(Some("0\n"), ""), State::Green);
        assert_eq!(state_of(None, ""), State::NotRun);
        assert_eq!(
            state_of(Some(""), ""),
            State::NotRun,
            "an empty status is no status"
        );
        assert_eq!(
            state_of(Some("101"), log),
            State::Red(101, vec!["a::two".to_owned(), "b::three".to_owned()])
        );
        let checks = plan(&tools_here(), &[], &[]);
        let rows = vec![
            (checks[0].clone(), State::Green, Some(3), String::new()),
            (
                checks[2].clone(),
                state_of(Some("101"), log),
                Some(900),
                tail(log, 2),
            ),
            (checks[3].clone(), State::NotRun, None, String::new()),
        ];
        let text = result("2026-10-13T12-00-00Z", &rows);
        assert!(
            text.starts_with("# Checks, 2026-10-13T12-00-00Z\n"),
            "{text}"
        );
        assert!(
            text.contains(
                "3 checks: 1 green, 1 red, 1 not run, 0 not run here (CI only). NOT GREEN"
            ),
            "{text}"
        );
        assert!(text.contains("| test | RED (exit 101) | 900 |"), "{text}");
        assert!(text.contains("| deny | NOT RUN | - |"), "{text}");
        assert!(text.contains("Failing tests: a::two, b::three"), "{text}");
        assert!(
            text.contains("test b::three ... FAILED\nfailures:"),
            "{text}"
        );
    }

    /// A WORKFLOW, READ: jobs, named and unnamed steps, folded blocks with
    /// blank lines and deeper indents, a one-line `run:`, `uses:` steps
    /// skipped; and each step becomes one check, the plan's own workspace
    /// steps and the CI-only jobs said as such.
    #[test]
    fn every_workflow_step_with_a_script_becomes_one_check() {
        let yml = "on: push\njobs:\n  purity:\n    runs-on: x\n    steps:\n      \
                   - uses: actions/checkout@v4\n      - name: Gate 0 — scan\n        \
                   run: |\n          set -euo pipefail\n\n          if true; then\n            \
                   echo deep\n          fi\n      - name: Gate 1\n        run: echo one\n  \
                   build:\n    steps:\n      - name: Gate 6a — formatting\n        \
                   if: x\n        run: cargo fmt --all --check\n      - run: echo unnamed\n  \
                   mutants:\n    steps:\n      - name: Test every mutation\n        run: |\n          \
                   cargo mutants\n";
        let yml = format!(
            "{yml}  cover:\n    steps:\n      - name: Measure\n        run: |\n{0}cargo install \
             cargo-llvm-cov --locked\n{0}cargo llvm-cov\n",
            " ".repeat(10)
        );
        let steps = ci_steps(&yml).expect("a workflow");
        let seen: Vec<(&str, &str)> = steps
            .iter()
            .map(|s| (s.job.as_str(), s.name.as_str()))
            .collect();
        assert_eq!(
            seen,
            [
                ("purity", "Gate 0 — scan"),
                ("purity", "Gate 1"),
                ("build", "Gate 6a — formatting"),
                ("build", "echo unnamed"),
                ("mutants", "Test every mutation"),
                ("cover", "Measure"),
            ]
        );
        assert_eq!(
            steps[0].script,
            "set -euo pipefail\n\nif true; then\n  echo deep\nfi\n"
        );
        assert_eq!(steps[1].script, "echo one\n");
        let checks = ci_checks(&steps);
        let names: Vec<&str> = checks.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "ci-purity-gate-0-scan",
                "ci-purity-gate-1",
                "ci-build-echo-unnamed",
                "ci-mutants-test-every-mutation",
                "ci-cover-measure"
            ],
            "the formatting step is the plan's own `fmt`"
        );
        assert_eq!(
            words(&checks[4].command),
            [
                "printf",
                "%s\\n",
                "not run here: it runs `cargo install`, which installs a tool into the user's own \
                 toolchain; the installed one is used by the plan's own checks",
                ";",
                "exit",
                "125"
            ],
            "{}",
            checks[4].command
        );
        assert!(
            checks[2].command.contains("checks_all\" step build 1 >"),
            "{}",
            checks[2].command
        );
        assert!(
            checks[2]
                .command
                .contains("bash --noprofile --norc -eo pipefail")
        );
        assert!(
            checks[2]
                .command
                .contains("GITHUB_ENV=\"$CHECKS_BIN/github-build.env\"")
                && checks[0]
                    .command
                    .contains("GITHUB_ENV=\"$CHECKS_BIN/github-purity.env\""),
            "one environment file a job, as one runner a job"
        );
        assert!(
            checks[3].command.ends_with("exit 125"),
            "{}",
            checks[3].command
        );
        assert_eq!(state_of(Some("125\n"), ""), State::NotHere);
        let back = read_plan(&plan_tsv(&checks)).expect("one line a check");
        assert_eq!(back, checks);
        assert!(ci_steps("on: push\n").is_err(), "no jobs");
        assert!(ci_steps("jobs:\n  a:\n    steps:\n      - uses: x\n").is_err());
        let rows = vec![
            (checks[0].clone(), State::Green, Some(1), String::new()),
            (checks[3].clone(), State::NotHere, Some(0), String::new()),
        ];
        let text = result("2026-10-14T00-00-00Z", &rows);
        assert!(
            text.contains("2 checks: 1 green, 0 red, 0 not run, 1 not run here (CI only). EVERY CHECK RUN HERE IS GREEN"),
            "{text}"
        );
        assert!(text.contains("| NOT RUN HERE (CI only) |"), "{text}");
    }

    /// The shell words of `command` as `bash` would split them, quotes
    /// removed, with each unquoted `;`, `&&`, `||`, `|`, `(` and `)` a word of
    /// its own. Enough of the grammar to prove what a plan line runs: single
    /// quotes take everything literally, double quotes honour `\\`, and a
    /// backslash outside quotes escapes the next character. Panics on an
    /// unclosed quote, which is the syntax error this exists to catch.
    fn words(command: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut word: Option<String> = None;
        let mut chars = command.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\'' => {
                    let w = word.get_or_insert_with(String::new);
                    loop {
                        match chars.next() {
                            Some('\'') => break,
                            Some(x) => w.push(x),
                            None => panic!("an unclosed single quote in: {command}"),
                        }
                    }
                }
                '"' => {
                    let w = word.get_or_insert_with(String::new);
                    loop {
                        match chars.next() {
                            Some('"') => break,
                            Some('\\') => match chars.next() {
                                Some(x @ ('"' | '\\' | '$' | '`')) => w.push(x),
                                Some(x) => {
                                    w.push('\\');
                                    w.push(x);
                                }
                                None => panic!("an unclosed double quote in: {command}"),
                            },
                            Some(x) => w.push(x),
                            None => panic!("an unclosed double quote in: {command}"),
                        }
                    }
                }
                '\\' => {
                    if let Some(x) = chars.next() {
                        word.get_or_insert_with(String::new).push(x);
                    }
                }
                ' ' => out.extend(word.take()),
                ';' | '(' | ')' => {
                    out.extend(word.take());
                    out.push(c.to_string());
                }
                '&' | '|' => {
                    out.extend(word.take());
                    if chars.peek() == Some(&c) {
                        let _second = chars.next();
                        out.push(format!("{c}{c}"));
                    } else {
                        out.push(c.to_string());
                    }
                }
                _ => word.get_or_insert_with(String::new).push(c),
            }
        }
        out.extend(word.take());
        out
    }

    /// **A DECLINED COMMAND STAYS INERT WHATEVER ITS REASON SAYS** (R3-14
    /// repair 1). The reason was pasted inside `echo '...'`: an apostrophe in
    /// it closed the quote, so the `npm ci` line was a syntax error (exit 2,
    /// RED instead of NOT RUN HERE) and the `cargo install` line ran
    /// `the` as a command. Every hostile character now stays inside one word.
    #[test]
    fn a_declined_command_stays_inert_whatever_its_reason_says() {
        for why in [
            "another checkout's",
            "it's `x`; rm -r \"$HOME\" && echo $(id) | sh",
            "'",
            "''\\'",
            "a\ttab and a\nline break",
            "",
        ] {
            let command = declined_command(why);
            assert!(!command.contains(['\t', '\n']), "{command:?}");
            let flat: String = why
                .chars()
                .map(|c| if matches!(c, '\t' | '\n') { ' ' } else { c })
                .collect();
            assert_eq!(
                words(&command),
                ["printf", "%s\\n", flat.as_str(), ";", "exit", "125"],
                "{command}"
            );
        }
    }

    /// **EVERY LINE OF THE PLAN IS ONE WELL-FORMED COMMAND** (R3-14 repair
    /// 1): over the real workflow, no command holds a tab or a line break
    /// (the plan's own separators), every quote closes, and every declined
    /// check is exactly a `printf` of its reason and `exit 125`. The tests
    /// before this checked only a declined line's first words.
    #[test]
    fn every_line_of_the_plan_is_one_well_formed_command() {
        let yml = std::fs::read_to_string(WORKFLOW).expect("run from the repository root");
        let steps = ci_steps(&yml).expect("the workflow reads");
        let checks = plan(&tools_here(), &steps, &["some_slow_test".to_owned()]);
        let mut declined = 0;
        for check in &checks {
            assert!(
                !check.command.contains(['\t', '\n', '\r']),
                "{}",
                check.name
            );
            let said = words(&check.command);
            if said.first().map(String::as_str) == Some("printf") {
                declined += 1;
                assert_eq!(said.len(), 6, "{}: {said:?}", check.name);
                assert_eq!(said[3..], [";", "exit", "125"], "{}", check.name);
                assert!(said[2].starts_with("not run here: "), "{}", check.name);
            }
        }
        assert!(
            declined >= 2,
            "cargo install and npm ci are declined: {declined}"
        );
        // EACH CI-ONLY JOB SAYS ITS OWN REASON (R3-14 repair 3): `ci-ok` was
        // declined as "mutation testing takes hours", which is not that job.
        let ci_ok: Vec<_> = checks
            .iter()
            .filter(|check| check.name.starts_with("ci-ci-ok-"))
            .collect();
        assert!(!ci_ok.is_empty(), "the ci-ok job is in the plan");
        for check in ci_ok {
            assert!(
                check.command.contains("other jobs") && !check.command.contains("mutation"),
                "{}: {}",
                check.name,
                check.command
            );
        }
        let read_back = read_plan(&plan_tsv(&checks)).expect("the plan reads back");
        assert_eq!(read_back, checks, "the plan survives its own file");
    }

    /// **A STEP RUNS WITH THE RUNNER'S FILES AND IDS** (R3-14 repair 1). The
    /// probe writes `$GITHUB_OUTPUT` and the coverage step names
    /// `$GITHUB_RUN_ID` and `$GITHUB_RUN_ATTEMPT`; unset here, both failed on
    /// the variable, and their REDs (and gate 20's after them) were the
    /// runner's, not the tree's.
    #[test]
    fn a_step_runs_with_the_runners_files_and_ids() {
        let step = CiStep {
            job: "cover".to_owned(),
            name: "Use a fresh coverage target".to_owned(),
            script: "echo \"$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT\" >> \"$GITHUB_OUTPUT\"\n"
                .to_owned(),
        };
        let command = &ci_checks(std::slice::from_ref(&step))[0].command;
        let said = words(command);
        for set in [
            "GITHUB_OUTPUT=$CHECKS_BIN/github-cover.output",
            "GITHUB_RUN_ID=$CHECKS_RUN_ID",
            "GITHUB_RUN_ATTEMPT=1",
            "GITHUB_ENV=$CHECKS_BIN/github-cover.env",
        ] {
            assert!(said.iter().any(|w| w == set), "{set} in {command}");
        }
        assert!(
            said.iter().any(|w| w == "$CHECKS_BIN/github-cover.output"),
            "the output file is made before the step: {command}"
        );
        assert!(
            RUNNER.contains("export CHECKS_RUN_ID=\"${CHECKS_RUN_ID:-$(date -u +%s)}\""),
            "the run id is set once a run"
        );
    }

    /// THE REAL WORKFLOW: every gate it runs is in the plan, gate 10 among
    /// them, and none is lost to a block the reader cut short.
    #[test]
    fn the_real_workflow_yields_every_gate() {
        let yml = std::fs::read_to_string(WORKFLOW).expect("run from the repository root");
        let steps = ci_steps(&yml).expect("the workflow reads");
        let named = yml
            .lines()
            .filter(|l| l.trim_start().starts_with("run:"))
            .count();
        assert_eq!(steps.len(), named, "one step a `run:` key");
        assert!(
            steps
                .iter()
                .any(|s| s.name.starts_with("Gate 10 ") && s.script.contains("invariant_paths")),
            "gate 10 is read whole"
        );
        let checks = plan(&tools_here(), &steps, &[]);
        let mut names: Vec<&str> = checks.iter().map(|c| c.name.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "every check's name is its own");
    }

    #[test]
    fn the_stamp_is_utc_and_file_name_safe() {
        assert_eq!(utc_stamp(0), "1970-01-01T00-00-00Z");
        assert_eq!(utc_stamp(951_782_400), "2000-02-29T00-00-00Z");
        assert_eq!(utc_stamp(1_791_895_000), "2026-10-13T12-36-40Z");
        assert_eq!(utc_stamp(4_102_444_799), "2099-12-31T23-59-59Z");
    }

    #[test]
    fn the_tools_are_read_from_the_tree() {
        let here = tools(Path::new(".")).expect("run from the repository root");
        assert!(here.contains(&"checks_all".to_owned()), "{here:?}");
        assert!(here.contains(&"gates_routes".to_owned()), "{here:?}");
        assert!(tools(Path::new("/nonexistent-checks-root")).is_err());
    }
}
