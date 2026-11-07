//! Writes every FIB-PREVIEW-REF golden scenario, `FORMAT.md`, `INDEX.md`
//! and `MANIFEST.md` into the directory named by the first argument.
//! PREVIEW: seeded synthetic data only; not a result.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(dir) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: fib_ref_golden <directory>");
        return ExitCode::from(2);
    };
    match fib_ref::golden::write_all(&dir) {
        Ok(n) => {
            println!("wrote {n} files into {}", dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("writing {}: {e}", dir.display());
            ExitCode::FAILURE
        }
    }
}
