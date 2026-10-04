#![cfg(test)]
//! Tests of `cli gdfl-import`. No vendor byte: the sources here are empty or
//! refused, and the import's own behaviour is tested in `pull::gdfl_import`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-cli-gdfl-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn env_of(pairs: &[(&str, PathBuf)]) -> impl Fn(&str) -> Option<OsString> {
    let pairs: Vec<(String, OsString)> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone().into_os_string()))
        .collect();
    move |var| pairs.iter().find(|(k, _)| k == var).map(|(_, v)| v.clone())
}

#[test]
fn a_day_is_exactly_yyyy_mm_dd() {
    assert_eq!(day("2024-04-01").unwrap().to_string(), "2024-04-01");
    for bad in [
        "2024-4-01",
        "24-04-01",
        "2024-04-1",
        "2024/04/01",
        "2024-04",
        "2024-04-01-1",
        "2024-0a-01",
        "2024-02-30",
    ] {
        assert!(day(bad).is_err(), "{bad}");
    }
}

#[test]
fn every_misuse_is_refused_before_anything_is_read() {
    let store = scratch("misuse");
    let env = env_of(&[]);
    let cases: [&[&str]; 5] = [
        &["indices", "tickstore", "2024-04-01"],
        &["futures", "tickstore", "2024-04-01", "2024-04-01"],
        &["indices", "tickstore", "2024-04-xx", "2024-04-01"],
        &["indices", "tickstore", "2024-04-01", "x"],
        &["indices", "ftp", "2024-04-01", "2024-04-01"],
    ];
    for case in cases {
        let mut out = String::new();
        assert_eq!(
            command_with(case, &env, &|| Ok(store.clone()), &mut out),
            MISUSED,
            "{case:?}"
        );
        assert!(out.starts_with("refused: "), "{out}");
    }
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &env_of(&[(TICKSTORE_VAR, store.clone())]),
        &|| Err("no store".to_owned()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(out.contains("no store"));
    std::fs::remove_dir_all(store).unwrap();
}

#[test]
fn a_root_variable_that_is_unset_or_empty_is_named_never_defaulted() {
    let store = scratch("unset");
    for (source, kind, var) in [
        ("tickstore", "indices", TICKSTORE_VAR),
        ("tickstore", "options", TICKSTORE_VAR),
        ("zip", "stocks", CM_ARCHIVE_VAR),
        ("zip", "options", NFO_ARCHIVES_VAR),
    ] {
        for env in [env_of(&[]), env_of(&[(var, PathBuf::new())])] {
            let mut out = String::new();
            let code = command_with(
                &[kind, source, "2024-04-01", "2024-04-01"],
                &env,
                &|| Ok(store.clone()),
                &mut out,
            );
            assert_eq!(code, FAILED, "{source} {kind}");
            assert!(out.contains(var) && out.contains("no default"), "{out}");
        }
    }
    std::fs::remove_dir_all(store).unwrap();
}

#[test]
fn an_empty_source_reports_every_day_it_does_not_hold() {
    let root = scratch("empty");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    // The smallest zip there is: an end record and nothing else.
    let mut empty_zip = vec![0x50, 0x4b, 0x05, 0x06];
    empty_zip.extend_from_slice(&[0; 18]);
    let cm = root.join("cm.zip");
    std::fs::write(&cm, empty_zip).unwrap();
    let env = env_of(&[
        (TICKSTORE_VAR, root.join("ts")),
        (CM_ARCHIVE_VAR, cm),
        (NFO_ARCHIVES_VAR, root.join("years")),
    ]);
    for (source, kind) in [
        ("tickstore", "indices"),
        ("tickstore", "options"),
        ("zip", "stocks"),
        ("zip", "options"),
    ] {
        let mut out = String::new();
        let code = command_with(
            &[kind, source, "2024-04-01", "2024-04-02", "NIFTY"],
            &env,
            &|| Ok(store.clone()),
            &mut out,
        );
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains("days not in source  2 2024-04-01 2024-04-02"),
            "{out}"
        );
        assert!(out.contains("1s only"), "{out}");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_named_failure_or_a_refused_run_exits_failed() {
    let root = scratch("failed");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let env = env_of(&[
        (TICKSTORE_VAR, root.join("ts")),
        (CM_ARCHIVE_VAR, root.join("absent.zip")),
    ]);
    // A disaster-recovery Saturday: a special session, refused by name.
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-03-02", "2024-03-02"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(
        out.contains("failures            1") && out.contains("calendar"),
        "{out}"
    );
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-04-02", "2024-04-01"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(
        out.contains("GDFL IMPORT REFUSED") && out.contains("backwards"),
        "{out}"
    );
    let mut out = String::new();
    let code = command_with(
        &["indices", "zip", "2024-04-01", "2024-04-01"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(out.contains("absent.zip"), "{out}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_dispatched_verb_reads_the_process_environment() {
    let mut out = String::new();
    let code = crate::run(
        &argv(&["gdfl-import", "futures", "zip", "2024-04-01", "2024-04-01"]),
        &mut out,
    );
    assert_eq!(code, MISUSED);
    assert!(out.contains("not a GDFL import kind"), "{out}");
    let mut out = String::new();
    assert_eq!(command(&["indices"], &mut out), MISUSED);
    // A closed day: the edge is read and nothing is written, whatever the
    // environment holds.
    let mut out = String::new();
    let _ = command(
        &["indices", "tickstore", "2024-04-06", "2024-04-06"],
        &mut out,
    );
    assert!(out.contains("GDFL IMPORT"), "{out}");
}
