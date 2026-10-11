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

fn env_of(pairs: &[(&str, PathBuf)]) -> impl Fn(&str) -> Option<OsString> + use<> {
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
    // A day before the calendar's spans, refused by name. (A special session
    // was this example until D-3319 read it inside its windows.)
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2017-12-29", "2017-12-29"],
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

/// D-3317: `--retry-refused` is a flag, never a symbol, so the journal key
/// stays `*`; the run's `run` line names the source this binary measured and
/// the receipt lists how each day closed.
#[test]
fn the_retry_flag_is_never_a_symbol_and_the_run_line_names_the_source() {
    let root = scratch("retry");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    std::fs::create_dir_all(&ts).unwrap();
    let env = env_of(&[(TICKSTORE_VAR, ts.clone())]);
    let mut out = String::new();
    let code = command_with(
        &[
            "indices",
            "tickstore",
            "2024-04-01",
            "2024-04-01",
            RETRY_REFUSED,
        ],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, OK, "{out}");
    for part in [
        "days not in source  1 2024-04-01",
        "days done, refused  0",
        "days incomplete     0",
        "days all refused    0",
    ] {
        assert!(out.contains(part), "{out} lacks {part:?}");
    }
    let journal = std::fs::read_to_string(pull::gdfl_import::journal_path(&store)).unwrap();
    let mut lines = journal.lines();
    let run = lines.next().unwrap();
    assert!(
        run.starts_with("run ")
            && run.contains(" source=tickstore ")
            && run.contains(" archive_bytes=0 ")
            && run.contains(" format_sha256=unmeasured"),
        "{journal}"
    );
    let commit = crate::commit_stamp().unwrap_or("unstamped");
    assert!(run.contains(&format!(" commit={commit} ")), "{run}");
    assert!(
        lines
            .next()
            .is_some_and(|l| l.starts_with("refused indices 2024-04-01 * ")
                && l.contains(" reason=day_not_held ")),
        "{journal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// D-3333: a capital-market run files the stored scope, built from the
/// options tree the store holds (every directory one underlying), and says
/// so; `--stored` is a flag, never a symbol, that narrows the run to what
/// D-3333 adds under a journal key of its own; on an options run it is
/// refused before anything is read.
#[test]
fn the_stored_flag_narrows_a_capital_market_run_to_what_d3333_adds() {
    let root = scratch("stored");
    let store = root.join("store");
    let tree = options_tree(&store);
    for name in ["HDFC", "NIFTY", "TATAMOTORS"] {
        std::fs::create_dir_all(tree.join(name)).unwrap();
    }
    // A file in the tree is not an underlying.
    std::fs::write(tree.join("README"), b"not a directory").unwrap();
    let ts = root.join("ts");
    std::fs::create_dir_all(&ts).unwrap();
    let env = env_of(&[(TICKSTORE_VAR, ts)]);
    assert_eq!(
        stored_option_underlyings(&store).unwrap(),
        vec!["HDFC", "NIFTY", "TATAMOTORS"]
    );
    assert!(
        stored_option_underlyings(&root.join("absent"))
            .unwrap()
            .is_empty()
    );
    let mut out = String::new();
    let code = command_with(
        &["stocks", "tickstore", "2024-04-01", "2024-04-01", STORED],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("stored (D-3333): 7 index rows and ")
            && out.contains("3 option underlyings read from "),
        "{out}"
    );
    let journal = std::fs::read_to_string(pull::gdfl_import::journal_path(&store)).unwrap();
    let line = journal
        .lines()
        .find(|l| l.starts_with("refused stocks 2024-04-01 "));
    assert!(line.is_some(), "{journal}");
    let line = line.unwrap();
    // The filter's key names the stored shares and where old names are
    // filed, never `*`: the swept surface's done days are not skipped.
    assert!(
        !line.contains(" * ") && line.contains("HDFC") && line.contains("TMPV"),
        "{line}"
    );
    let (_, filter) = scope_of(ImportKind::Indices, &store).unwrap();
    assert_eq!(filter, 0, "an index run reads no options tree");
    let (scope, _) = scope_of(ImportKind::Indices, &store).unwrap();
    assert_eq!(
        stored_filter(ImportKind::Indices, &scope),
        vec![
            "INDIAVIX",
            "NIFTYCPSE",
            "NIFTYFINSERVICE",
            "NIFTYFPI150",
            "NIFTYIT",
            "NIFTYMIDSELECT",
            "NIFTYNEXT50"
        ]
    );
    let mut out = String::new();
    let code = command_with(
        &["options", "tickstore", "2024-04-01", "2024-04-01", STORED],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, MISUSED);
    assert!(
        out.starts_with("refused: ") && out.contains(STORED),
        "{out}"
    );
    // An underlying the store could not file under its own spelling refuses
    // the run by name before any day is read. (Lower case, and no upper-case
    // twin in the tree: a case-insensitive volume would fold the two.)
    std::fs::create_dir_all(tree.join("acc")).unwrap();
    let mut out = String::new();
    let code = command_with(
        &["stocks", "tickstore", "2024-04-01", "2024-04-01"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(out.contains("\"acc\" is not a share symbol"), "{out}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A run that back-filled days inside held months says how many month files
/// it repaired to a new generation, and where each is journaled (D-3350;
/// DB-N26, WP-12d); a run that repaired none says 0.
#[test]
fn the_receipt_counts_the_months_a_back_fill_repaired() {
    for (repaired, line) in [
        (
            24,
            "  months repaired     24 (a new generation each; audit/gdfl.repairs)\n",
        ),
        (
            0,
            "  months repaired     0 (a new generation each; audit/gdfl.repairs)\n",
        ),
    ] {
        let report = Report {
            months_repaired: repaired,
            ..Report::default()
        };
        let mut out = String::new();
        render(
            &mut out,
            ImportKind::Indices,
            "zip",
            Path::new("store"),
            &report,
        );
        assert!(out.contains(line), "{out}");
    }
}

/// An ex-date event the options import detected is on the receipt, with the
/// ledger lines it wrote (D-3355; DB-B13): before, a detected re-listing
/// showed nowhere a person reads. A run that found none says 0.
#[test]
fn the_receipt_counts_the_adjustments_and_the_ledger_lines_they_wrote() {
    let report = Report {
        adjustments: 2,
        lineage_lines: 41,
        lot_lines: 7,
        ..Report::default()
    };
    let mut out = String::new();
    render(
        &mut out,
        ImportKind::Options,
        "zip",
        Path::new("store"),
        &report,
    );
    for line in [
        "  adjustments found   2 (ex-date events; imports/gdfl.lineage)\n",
        "  lineage records     41\n",
        "  lot records         7\n",
    ] {
        assert!(out.contains(line), "{out}");
    }
    let mut none = String::new();
    render(
        &mut none,
        ImportKind::Options,
        "zip",
        Path::new("store"),
        &Report::default(),
    );
    assert!(
        none.contains("  adjustments found   0 (ex-date events; imports/gdfl.lineage)\n"),
        "{none}"
    );
}
