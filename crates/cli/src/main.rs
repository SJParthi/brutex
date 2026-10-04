//! The operator entry point for the sweep.
//!
//! This file holds one call and one pure function, on purpose. A `main` cannot
//! be entered by a unit test — `cargo test` builds the binary with its own
//! harness and never runs it — so every line of logic left here is a line no
//! test can reach. Everything therefore lives in [`cli`], which is ordinary
//! library code, and `tests/binary.rs` runs this binary so that even this call
//! is measured rather than assumed. The shape is `crates/api/src/main.rs`'s,
//! and deliberately so: two binaries that end differently are two things an
//! operator has to learn.
//!
//! # The four commands
//!
//! ```text
//! cli sweep        SESSIONS MIN_HITS                          generated bars
//! cli audit        SESSIONS MIN_HITS                          generated bars
//! cli auto         SESSIONS                                   generated bars
//! cli sweep-stored VENDOR UNDERLYING RUNG YEAR MONTH MIN_HITS  REAL bars
//! ```
//!
//! This list read `sweep | auto` and omitted half the surface, including the
//! only command that touches real market data. `sweep-stored` also needs the
//! binary stamped from a clean HEAD. The build proves HEAD, index, and working
//! source agree; an explicit `BRUTEX_COMMIT` can only assert that same HEAD and
//! cannot bypass a dirty tree. CLAUDE.md section 3 rule 3 forbids a computation
//! whose run identity cannot be recorded, so an unproved build refuses before it
//! reads a bar. [`cli::USAGE`] is what an operator actually sees; a doc comment
//! here reaches nobody at a terminal, which is why it had drifted unnoticed.

// A BINARY IS ITS OWN CRATE ROOT. `lib.rs` carries this attribute and it does
// not reach here, so CI gate 16 checks every root separately.
#![forbid(unsafe_code)]

/// Runs one command, prints what it produced, and returns the shell's code.
///
/// The output is accumulated into a `String` by [`cli::run`] and printed here
/// rather than written as it is produced, so that every arm of the library is
/// drivable from a test with no stdout to capture.
fn main() -> std::process::ExitCode {
    // WRITTEN, NEVER PRINTED (v53-2, D-1484). `println!` panics on a closed
    // stdout, so `cli sweep 6 100 | true` exited 101 after the work was done.
    // `cli::deliver` writes, says a failure on stderr, and decides the code.
    let (mut stdout, mut stderr) = (std::io::stdout(), std::io::stderr());
    // THE STORE IS PROVED BEFORE THE FIRST POSSIBLE WRITE.
    //
    // `preflight_store_root` does not create the resolved path and does not
    // canonicalize-or-fallback. A removed external volume, a dangling symlink,
    // or a regular file therefore stops here: no log sink is installed and no
    // command is dispatched against a different directory.
    if let Err(why) = cli::preflight_store_root() {
        let refused = format!("refused: {why}\n");
        let code = cli::deliver(cli::FAILED, &refused, &mut stdout, &mut stderr);
        return std::process::ExitCode::from(code);
    }

    // THE SINK, INSTALLED BEFORE THE COMMAND RUNS AND NOWHERE ELSE.
    //
    // Here rather than inside `cli::run` because the sink is process-wide:
    // `telemetry::install` refuses a second call, so a `run` that installed
    // would work once and refuse for the rest of the process — and the test
    // binary calls `run` many times. `tests/binary.rs` executes THIS file, so
    // the line is measured rather than assumed, which is the same reason the
    // `run` call below lives here alone.
    //
    // A failure is PRINTED, not fatal. A sweep with no log is still a correct
    // sweep, and refusing to compute because a directory is unwritable trades a
    // whole answer for an audit trail. `CLAUDE.md` §4 bans a fallback that
    // HIDES a failure; this one names it, above the report, on the same screen.
    let where_events_went = cli::install_log();
    cli::announce_unlogged(&where_events_went, &mut stderr);

    // `args_os`, never `args`: the latter panics on an argument that is not
    // valid UTF-8, exiting 101 before the line below can say where events
    // went. The library refuses such a word as a misuse instead. D-0998.
    let mut out = String::new();
    let code = cli::run_durable_os(std::env::args_os().skip(1), &mut out);
    // PRINTED ON SUCCESS TOO, and that is the change. It used to print only on
    // failure, so a run whose events landed somewhere `/logs` does not read
    // looked exactly like a run that was fully observable. `install_log`'s doc
    // carries the measurement.
    let shown = format!("{where_events_went}\n{out}");
    std::process::ExitCode::from(cli::deliver(code, &shown, &mut stdout, &mut stderr))
}
