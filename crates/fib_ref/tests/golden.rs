//! The committed golden files are exactly what this crate generates, byte for
//! byte: every scenario, `FORMAT.md`, `INDEX.md` and `MANIFEST.md`, and no
//! other file. Idempotence (same seeds, same bytes) and the manifest's sums
//! are both checked here. PREVIEW: synthetic data only.

#![allow(clippy::panic, clippy::expect_used, reason = "a test that cannot panic cannot fail")]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reports/fib-preview-ref-20261107/golden")
}

#[test]
fn committed_golden_files_are_regenerated_byte_for_byte() {
    let dir = golden_dir();
    let files = fib_ref::golden::files();
    assert!(files.len() >= 203, "{} files", files.len());
    for (name, text) in &files {
        let on_disk = fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(on_disk == *text, "{name} differs from what the crate generates");
    }
    let manifest = fs::read_to_string(dir.join("MANIFEST.md")).expect("MANIFEST.md");
    assert_eq!(manifest, fib_ref::golden::manifest(&files));
    let mut expected: BTreeSet<String> = files.iter().map(|(n, _)| n.clone()).collect();
    expected.insert("MANIFEST.md".to_owned());
    let present: BTreeSet<String> = fs::read_dir(&dir)
        .expect("golden dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(present, expected, "files in the golden directory that the crate does not write, or missing ones");
}

#[test]
fn every_manifest_line_is_the_sha256_of_its_file() {
    let dir = golden_dir();
    let manifest = fs::read_to_string(dir.join("MANIFEST.md")).expect("MANIFEST.md");
    let mut checked = 0;
    for line in manifest.lines() {
        let Some((sum, name)) = line.split_once("  ") else { continue };
        if sum.len() != 64 {
            continue;
        }
        let bytes = fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(fib_ref::golden::hex(&fib_ref::golden::sha256(&bytes)), sum, "{name}");
        checked += 1;
    }
    assert!(checked >= 203, "only {checked} manifest lines checked");
}
