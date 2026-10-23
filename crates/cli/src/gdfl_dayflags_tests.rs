#![cfg(test)]
//! Tests of `cli gdfl-dayflags`. No vendor byte: the sources here are empty
//! or missing, and the rescan's own counting is tested in
//! `pull::gdfl_import` on invented files.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-dayflags-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn env_of(pairs: &[(&str, PathBuf)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let pairs: Vec<(String, OsString)> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone().into_os_string()))
        .collect();
    move |var| pairs.iter().find(|(k, _)| k == var).map(|(_, v)| v.clone())
}

/// Every file under `dir`, relative path to bytes.
fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push((
                    path.strip_prefix(dir).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_misuse_is_refused_before_anything_is_read() {
    let store = scratch("misuse");
    let env = env_of(&[]);
    let cases: [&[&str]; 8] = [
        &[],
        &["indices", "2024-04-01"],
        &["futures", "2024-04-01", "2024-04-01"],
        &["indices", "2024-04-xx", "2024-04-01"],
        &["indices", "2024-04-02", "2024-04-01"],
        &["indices", "2024-04-01", "2024-04-01", "--lineage"],
        &["options", "2024-04-01", "2024-04-01", "--lineage", "x"],
        &["options", "2024-04-01", "2024-04-01", "NIFTY"],
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
    assert!(tree(&store).is_empty(), "a misuse writes nothing");
    std::fs::remove_dir_all(store).unwrap();
}

#[test]
fn the_zip_root_is_named_never_defaulted() {
    let store = scratch("unset");
    for (kind, var) in [
        ("indices", CM_ARCHIVE_VAR),
        ("stocks", CM_ARCHIVE_VAR),
        ("options", NFO_ARCHIVES_VAR),
    ] {
        let mut out = String::new();
        let code = command_with(
            &[kind, "2024-04-01", "2024-04-01"],
            &env_of(&[]),
            &|| Ok(store.clone()),
            &mut out,
        );
        assert_eq!(code, FAILED, "{kind}");
        assert!(out.contains(var), "{kind}: {out}");
        assert!(out.contains("no default"), "{out}");
    }
    let mut out = String::new();
    let missing = store.join("no-such.zip");
    let code = command_with(
        &["indices", "2024-04-01", "2024-04-01"],
        &env_of(&[(CM_ARCHIVE_VAR, missing)]),
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(out.starts_with("GDFL DAYFLAGS REFUSED: "), "{out}");
    assert!(tree(&store).is_empty(), "a refusal writes nothing");
    std::fs::remove_dir_all(store).unwrap();
}

/// Only days the journal records as imported are read; one the zips no
/// longer hold is named; a day never imported is counted and not read; the
/// journal and the bars are left byte for byte as they were.
#[test]
fn only_imported_days_are_read_and_nothing_but_the_ledger_is_written() {
    let store = scratch("days");
    let zips = scratch("zips");
    let journal = "run 7 source=zip archive=x archive_bytes=1 commit=unstamped started=7\n\
                   begin options 2024-04-01 *\n\
                   done options 2024-04-01 * definition=3 files=1 seconds=1 failures=0 late=0 \
                   late_unresolved=0 max_back_s=0 run=7\n\
                   refused options 2024-04-02 * definition=3 stage=vendor reason=day_not_held \
                   failures=0 run=7\n";
    std::fs::create_dir_all(store.join("imports")).unwrap();
    std::fs::write(store.join("imports/gdfl.journal"), journal).unwrap();
    std::fs::create_dir_all(store.join("bars/gdfl/NSE/FNO/NIFTY")).unwrap();
    std::fs::write(store.join("bars/gdfl/NSE/FNO/NIFTY/keep"), b"untouched").unwrap();
    let before = tree(&store);
    let mut out = String::new();
    let code = command_with(
        &["options", "2024-04-01", "2024-04-03", "--lineage"],
        &env_of(&[(NFO_ARCHIVES_VAR, zips.clone())]),
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("days read           0"), "{out}");
    assert!(out.contains("days not imported   2"), "{out}");
    assert!(out.contains("days not in source  1 2024-04-01"), "{out}");
    assert!(out.contains("flags lines written 0"), "{out}");
    assert!(out.contains("lineage records     0"), "{out}");
    // Nothing of the data was written: the journal and the bars are byte for
    // byte as they were. What the run's own locks leave (IA-16, D-6900) is
    // its released claim (an empty lock file) and its one runs-ledger id.
    let locks = |path: &Path| {
        let name = path.to_string_lossy();
        name.ends_with(".lock") || name.contains("runs.ledger")
    };
    let data = |all: Vec<(PathBuf, Vec<u8>)>| -> Vec<(PathBuf, Vec<u8>)> {
        all.into_iter().filter(|(path, _)| !locks(path)).collect()
    };
    assert_eq!(
        data(tree(&store)),
        data(before),
        "nothing of the data was written"
    );
    assert_eq!(
        std::fs::read(store.join("imports/gdfl.lock")).unwrap(),
        b"",
        "the claim was released"
    );
    let ledger = pull::gdfl_run::ledger::read_all(&store).unwrap();
    let last = ledger.last().unwrap().as_ref().unwrap();
    assert_eq!(
        (last.mode.as_str(), last.kind.as_str()),
        ("dayflags", "options")
    );
    std::fs::remove_dir_all(store).unwrap();
    std::fs::remove_dir_all(zips).unwrap();
}

/// IA-16 (D-6900 keep item 1; design §5.3; the cli half of IA-GR-14, whose
/// import half is pull's): while the kind's locks are held the rescan is
/// refused `run_busy` naming the holder's run, reads no day and writes no
/// ledger line; once they are released it rescans.
#[test]
fn the_rescan_is_refused_by_name_while_an_import_of_its_kind_runs() {
    let store = scratch("busy");
    let zips = scratch("busy-zips");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    std::fs::write(store.join("imports/gdfl.journal"), "").unwrap();
    // The holder: the kind's locks, as a run of it holds them (the import
    // half is pull's IA-GR-14).
    let run =
        pull::gdfl_run::claim_dayflags(&store, pull::gdfl_import::ImportKind::Options).unwrap();
    let mut out = String::new();
    let code = command_with(
        &["options", "2024-04-01", "2024-04-01"],
        &env_of(&[(NFO_ARCHIVES_VAR, zips.clone())]),
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.starts_with("GDFL DAYFLAGS REFUSED: run_busy: ")
            && out.contains(&format!("run={} ", run.run))
            && out.contains("read and wrote nothing"),
        "{out}"
    );
    assert!(!out.contains("run_busy: run_busy"), "the code once: {out}");
    assert!(!store.join("imports/gdfl.dayflags").exists(), "{out}");
    run.release().unwrap();
    let mut out = String::new();
    let code = command_with(
        &["options", "2024-04-01", "2024-04-01"],
        &env_of(&[(NFO_ARCHIVES_VAR, zips.clone())]),
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("days not imported   1"), "{out}");
    std::fs::remove_dir_all(store).unwrap();
    std::fs::remove_dir_all(zips).unwrap();
}
