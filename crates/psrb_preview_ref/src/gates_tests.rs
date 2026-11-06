//! The crate's global gates (TEST-PLAN section 2.0), read from its own files.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn crate_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    files(&root.join("src"), &mut out);
    files(&root.join("benches"), &mut out);
    out.push(root.join("Cargo.toml"));
    out
}

/// Every source file but this one (whose token lists would match themselves).
fn sources() -> Vec<(PathBuf, String)> {
    crate_files()
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs") && !p.ends_with("gates_tests.rs"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect()
}

/// T-G-1: no floating-point type anywhere in the crate (a cast to one must name it; the
/// workspace's `float_arithmetic` deny catches the arithmetic).
#[test]
fn t_g1_no_float_token_anywhere() {
    let tokens = [["f", "32"].concat(), ["f", "64"].concat()];
    for (path, text) in sources() {
        for t in &tokens {
            assert!(!text.contains(t.as_str()), "{} holds {t}", path.display());
        }
    }
}

/// T-G-2: no `as` cast and no indexing allowance in production code.
#[test]
fn t_g2_no_as_casts_no_indexing_allowance_in_production() {
    let lib = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap();
    assert!(lib.contains(&["#![deny(clippy::", "as_conversions)]"].concat()));
    for (path, text) in sources().into_iter().filter(|(p, _)| !p.to_string_lossy().ends_with("_tests.rs")) {
        assert!(!text.contains(&["allow(clippy::", "indexing_slicing"].concat()), "{}", path.display());
    }
}

/// T-G-3: nothing deletes, renames or truncates a file; every write is `create_new`.
#[test]
fn t_g3_no_delete_no_overwrite_api() {
    let banned = [
        ["remove", "_file"].concat(),
        ["remove", "_dir"].concat(),
        ["fs::", "rename"].concat(),
        ["File::", "create"].concat(),
        ["truncate", "(true)"].concat(),
    ];
    for (path, text) in sources() {
        for b in &banned {
            assert!(!text.contains(b.as_str()), "{} holds {b}", path.display());
        }
    }
}

/// T-G-5: the crate tracks only `.rs`, `.toml` and `.md` files (CLAUDE.md section 2).
#[test]
fn t_g5_only_rust_toml_md_files() {
    for p in crate_files() {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(["rs", "toml", "md"].contains(&ext), "{}", p.display());
    }
}

/// T-G-6: one common path: the rule sets' names appear only in their table and in tests.
#[test]
fn t_g6_one_common_path() {
    let names = [["VIDEO", "_ORIGINAL"].concat(), ["AUTHOR", "_V2"].concat(), ["VIDEO", "_SCREEN"].concat()];
    for (path, text) in sources() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name == "ruleset.rs" || name.ends_with("_tests.rs") || name == "lib.rs" {
            continue;
        }
        for n in &names {
            assert!(!text.contains(n.as_str()), "{name} names {n}");
        }
    }
}
