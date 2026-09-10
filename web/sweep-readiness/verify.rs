//! Repeatable local sweep checks. No stored-data sweep or vendor pull is started.
//! Build from the repository root with the command in README.md.
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct Check {
    name: &'static str,
    command: &'static str,
    args: &'static [&'static str],
    directory: &'static str,
}

const CHECKS: &[Check] = &[
    Check {
        name: "Comparison matches the reviewed report",
        command: "node",
        args: &["sweep-readiness/build-review.mjs", "--check"],
        directory: "web",
    },
    Check {
        name: "Workspace formatting",
        command: "cargo",
        args: &["fmt", "--all", "--check"],
        directory: ".",
    },
    Check {
        name: "Sweep tests",
        command: "cargo",
        args: &[
            "test",
            "-p",
            "vocab",
            "-p",
            "indicators",
            "-p",
            "engine",
            "-p",
            "runner",
            "--locked",
        ],
        directory: ".",
    },
    Check {
        name: "Result persistence regressions",
        command: "cargo",
        args: &["test", "-p", "cli", "--lib", "result", "--locked"],
        directory: ".",
    },
    Check {
        name: "Stored expression evidence",
        command: "cargo",
        args: &["test", "-p", "cli", "--lib", "expression", "--locked"],
        directory: ".",
    },
    Check {
        name: "Durable checkpoint journal",
        command: "cargo",
        args: &[
            "test",
            "-p",
            "cli",
            "--lib",
            "search_checkpoint",
            "--locked",
        ],
        directory: ".",
    },
    Check {
        name: "Stored AND continuation",
        command: "cargo",
        args: &["test", "-p", "cli", "--lib", "and_checkpoint", "--locked"],
        directory: ".",
    },
    Check {
        name: "Exact evaluated-candidate trades",
        command: "cargo",
        args: &["test", "-p", "cli", "--lib", "candidate_trades", "--locked"],
        directory: ".",
    },
    Check {
        name: "Selection V6 authority",
        command: "cargo",
        args: &["test", "-p", "cli", "--lib", "selection_v6", "--locked"],
        directory: ".",
    },
    Check {
        name: "Global Replay V4 lifecycle",
        command: "cargo",
        args: &["test", "-p", "cli", "--lib", "global_replay_v4", "--locked"],
        directory: ".",
    },
    Check {
        name: "Sweep lifecycle and depth evidence",
        command: "cargo",
        args: &["test", "-p", "cli", "--test", "sweep_evidence", "--locked"],
        directory: ".",
    },
    Check {
        name: "Sweep evidence API",
        command: "cargo",
        args: &["test", "-p", "api", "--lib", "sweepevidence", "--locked"],
        directory: ".",
    },
    Check {
        name: "Candidate detail API",
        command: "cargo",
        args: &["test", "-p", "api", "--lib", "candidatejson", "--locked"],
        directory: ".",
    },
    Check {
        name: "Expression search snapshot API",
        command: "cargo",
        args: &[
            "test",
            "-p",
            "api",
            "--lib",
            "expressionsearchjson",
            "--locked",
        ],
        directory: ".",
    },
    Check {
        name: "Sweep lint checks",
        command: "cargo",
        args: &[
            "clippy",
            "-p",
            "vocab",
            "-p",
            "indicators",
            "-p",
            "engine",
            "-p",
            "runner",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        directory: ".",
    },
    Check {
        name: "Workspace integration",
        command: "cargo",
        args: &["test", "--workspace", "--locked"],
        directory: ".",
    },
    Check {
        name: "Workspace lint checks",
        command: "cargo",
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        directory: ".",
    },
    Check {
        name: "Dependency policy",
        command: "cargo",
        args: &["deny", "check"],
        directory: ".",
    },
    Check {
        name: "All browser regressions",
        command: "npm",
        args: &["test"],
        directory: "web",
    },
    Check {
        name: "Browser type checks",
        command: "npm",
        args: &["run", "check"],
        directory: "web",
    },
    Check {
        name: "Browser production build",
        command: "npm",
        args: &["run", "build"],
        directory: "web",
    },
];

fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// A change detector only. This is NOT a strategy RunId or a provenance seal.
fn source_fingerprint() -> io::Result<String> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other("cannot enumerate source"));
    }
    let mut paths: Vec<_> = output
        .stdout
        .split(|b| *b == 0)
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .filter(|s| {
            s.starts_with("crates/")
                || s.starts_with("docs/")
                || s.starts_with("config/")
                || s.starts_with(".cargo/")
                || s.starts_with("web/src/")
                || s.starts_with("web/tests/")
                || s.starts_with("web/policy-guide/")
                || matches!(
                    *s,
                    "Cargo.toml"
                        | "Cargo.lock"
                        | "AGENTS.md"
                        | ".github/workflows/ci.yml"
                        | "deny.toml"
                        | "rust-toolchain"
                        | "rust-toolchain.toml"
                        | "web/package.json"
                        | "web/package-lock.json"
                        | "web/tsconfig.json"
                        | "web/svelte.config.js"
                        | "web/vite.config.js"
                        | "web/masters.js"
                        | "web/typeahead.js"
                        | "web/sweep-readiness/verify.rs"
                        | "web/sweep-readiness/build-review.mjs"
                        | "web/sweep-readiness/index.html"
                        | "web/sweep-readiness/README.md"
                        | "web/sweep-readiness/probes/support_lanes.rs"
                        | "web/sweep-readiness/deployment-preflight.rs"
                        | "web/sweep-readiness/deployment-preflight-tests.rs"
                        | "web/sweep-readiness/deployment-gates.rs"
                        | "web/sweep-readiness/deployment-files.rs"
                        | "web/sweep-readiness/deployment-preflight.md"
                )
        })
        .collect();
    paths.sort_unstable();
    paths.dedup();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for path in paths {
        for byte in path.as_bytes().iter().copied().chain([0]) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
        match File::open(path) {
            Ok(mut file) => {
                let mut buffer = [0; 8192];
                loop {
                    let read = file.read(&mut buffer)?;
                    if read == 0 {
                        break;
                    }
                    for byte in &buffer[..read] {
                        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => hash ^= u64::MAX,
            Err(error) => return Err(error),
        }
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    Ok(format!("fnv1a-change-detector:{hash:016x}"))
}

fn publish(
    run: &str,
    fingerprint: &str,
    complete: bool,
    stable: bool,
    rows: &[String],
) -> io::Result<()> {
    let contents = format!(
        "window.BRUTEX_AUDIT_CHECKS = {{\"run\":{},\"fingerprint\":{},\"complete\":{complete},\"stable\":{stable},\"checks\":[{}]}};\n",
        quoted(run),
        quoted(fingerprint),
        rows.join(",")
    );
    let staged = Path::new("web/sweep-readiness/evidence.next.js");
    fs::write(staged, &contents)?;
    fs::rename(staged, "web/sweep-readiness/evidence.js")?;
    Ok(())
}

fn verify(focused: bool) -> io::Result<bool> {
    if !Path::new("crates/engine/Cargo.toml").is_file() {
        return Err(io::Error::other(
            "run this tool from the brutex repository root",
        ));
    }
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    let run = format!(
        "{}{}-{}",
        if focused { "focused-" } else { "" },
        started.as_secs(),
        started.subsec_nanos()
    );
    let directory = PathBuf::from("target/sweep-readiness").join(&run);
    fs::create_dir_all(&directory)?;
    let fingerprint = source_fingerprint()?;
    let mut rows = Vec::new();
    let mut all_passed = true;
    let mut stable = true;
    publish(&run, &fingerprint, false, true, &rows)?;
    for (index, check) in CHECKS.iter().enumerate() {
        if focused && check.name.starts_with("Workspace") {
            continue;
        }
        // The workspace test command includes every focused Rust test target.
        // Do not repeat those targets in a full run: publishing this report can
        // trigger provenance rebuilds, turning duplicate checks into costly
        // repeated compiles. The focused mode retains each diagnostic group.
        if !focused
            && !check.name.starts_with("Workspace")
            && !matches!(
                check.name,
                "Comparison matches the reviewed report"
                    | "Dependency policy"
                    | "All browser regressions"
                    | "Browser type checks"
                    | "Browser production build"
            )
        {
            continue;
        }
        println!("Checking {}…", check.name);
        let log_path = directory.join(format!("check-{index}.log"));
        let mut log = File::create_new(&log_path)?;
        writeln!(
            log,
            "{} {} (directory {})",
            check.command,
            check.args.join(" "),
            check.directory
        )?;
        let clock = Instant::now();
        let outcome = Command::new(check.command)
            .args(check.args)
            .current_dir(check.directory)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log.try_clone()?))
            .status();
        let (passed, exit, note) = match outcome {
            Ok(status) => (
                status.success(),
                status.code().map_or("null".into(), |code| code.to_string()),
                String::new(),
            ),
            Err(error) => {
                writeln!(log, "Could not start: {error}")?;
                (false, "null".into(), error.to_string())
            }
        };
        all_passed &= passed;
        let seconds = clock.elapsed().as_secs();
        writeln!(log, "\nAudit exit: {exit}; elapsed: {seconds} seconds")?;
        log.sync_all()?;
        rows.push(format!("{{\"name\":{},\"passed\":{passed},\"exit\":{exit},\"seconds\":{seconds},\"note\":{},\"log\":{}}}", quoted(check.name), quoted(&note), quoted(&format!("../../{}", log_path.display()))));
        stable &= source_fingerprint()? == fingerprint;
        publish(&run, &fingerprint, false, stable, &rows)?;
        println!(
            "{}: {} ({seconds}s)",
            check.name,
            if passed {
                "passed"
            } else {
                "failed; read saved evidence"
            }
        );
    }
    stable &= source_fingerprint()? == fingerprint;
    publish(&run, &fingerprint, true, stable, &rows)?;
    fs::copy(
        "web/sweep-readiness/evidence.js",
        directory.join("evidence.js"),
    )?;
    println!(
        "Evidence saved in {}. No source difference detected at checkpoints: {stable}.",
        directory.display()
    );
    Ok(all_passed && stable)
}

fn main() -> std::process::ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--purity") {
        return match language_paths() {
            Ok(true) => std::process::ExitCode::SUCCESS,
            Ok(false) => std::process::ExitCode::FAILURE,
            Err(error) => {
                eprintln!("Language-path audit refused: {error}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    match verify(std::env::args().nth(1).as_deref() == Some("--focused")) {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Audit refused: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn language_paths() -> io::Result<bool> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other("git file enumeration failed"));
    }
    let mut checked = 0_usize;
    let mut valid = true;
    for bytes in output.stdout.split(|byte| *byte == 0) {
        if bytes.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(bytes).map_err(io::Error::other)?;
        if path.starts_with("web/") {
            continue;
        }
        checked += 1;
        let file = Path::new(path);
        let named = matches!(
            file.file_name().and_then(|n| n.to_str()),
            Some("LICENSE" | "CODEOWNERS" | ".gitignore" | ".gitattributes")
        );
        let allowed = match file.extension().and_then(|ext| ext.to_str()) {
            Some("rs" | "toml" | "md" | "lock" | "html" | "css") => true,
            Some("yml") => path.starts_with(".github/"),
            Some("json") => path.starts_with(".claude/"),
            _ => false,
        };
        if !named && !allowed {
            eprintln!("Forbidden path outside web/: {path}");
            valid = false;
        }
    }
    println!(
        "Checked {checked} tracked and nonignored untracked paths outside web/. Extension boundary: {}. This does not inspect dependency implementations or prove front-end-toolchain independence.",
        if valid { "passed" } else { "failed" }
    );
    Ok(valid && checked > 0)
}
