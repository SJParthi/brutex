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
//! Usage: `cli sweep SESSIONS MIN_HITS | cli auto SESSIONS`.

// A BINARY IS ITS OWN CRATE ROOT. `lib.rs` carries this attribute and it does
// not reach here, so CI gate 16 checks every root separately.
#![forbid(unsafe_code)]

/// Runs one command, prints what it produced, and returns the shell's code.
///
/// The output is accumulated into a `String` by [`cli::run`] and printed here
/// rather than written as it is produced, so that every arm of the library is
/// drivable from a test with no stdout to capture.
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = String::new();
    let code = cli::run(&args, &mut out);
    print!("{out}");
    std::process::ExitCode::from(code)
}
