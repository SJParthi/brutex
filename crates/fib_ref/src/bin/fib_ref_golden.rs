//! Writes every FIB-PREVIEW-REF golden scenario, `FORMAT.md`, `INDEX.md`
//! and `MANIFEST.md` into the directory named by the first argument.
//! PREVIEW: seeded synthetic data only; not a result. The behaviour is
//! `fib_ref::golden::writer_main`, unit-tested there.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    ExitCode::from(fib_ref::golden::writer_main(
        &args,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    ))
}
