#![cfg(test)]
//! Tests of `cli gdfl-import`. No vendor byte: the sources here are empty or
//! refused, and the import's own behaviour is tested in `pull::gdfl_import`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

pub(super) fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-cli-gdfl-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `dir` made a tick store the door accepts (AF-01): a directory with a
/// `FORMAT.md` at its root. The words are invented; no day file is in it.
pub(super) fn tick_store(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("FORMAT.md"),
        b"# synthetic tick store (cli test)\n",
    )
    .unwrap();
}

/// The environment `pairs` name, and the deliberate allowance a test build
/// needs: a test binary is often built from a dirty tree, so it carries no
/// commit stamp, and an import refuses that unless allowed (DB-B71).
pub(super) fn env_of(pairs: &[(&str, PathBuf)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let mut pairs: Vec<(String, OsString)> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone().into_os_string()))
        .collect();
    pairs.push((ALLOW_UNSTAMPED_VAR.to_owned(), OsString::from("1")));
    move |var| pairs.iter().find(|(k, _)| k == var).map(|(_, v)| v.clone())
}

/// **AN UNSTAMPED BUILD DOES NOT WRITE THE STORE UNLESS TOLD TO (DB-B71).**
/// A dirty-tree import wrote 14 `run ... commit=unstamped` journal lines on a
/// test shadow and nothing stopped it. Both arms of the rule, as a function of
/// the stamp and the allowance; and the verb itself, on whatever this test
/// binary is.
#[test]
fn an_unstamped_build_is_refused_a_store_write_unless_allowed() {
    let stamp = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(unstamped_write_refusal(Some(stamp), None), None);
    let why = unstamped_write_refusal(None, None).expect("refused");
    assert!(
        why.contains(ALLOW_UNSTAMPED_VAR) && why.contains("DB-B71"),
        "{why}"
    );
    assert!(unstamped_write_refusal(None, Some(std::ffi::OsStr::new("0"))).is_some());
    assert_eq!(
        unstamped_write_refusal(None, Some(std::ffi::OsStr::new("1"))),
        None
    );

    let root = scratch("unstamped");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let ts_var = ts.into_os_string();
    let bare = move |var: &str| (var == TICKSTORE_VAR).then(|| ts_var.clone());
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &bare,
        &|| Ok(store.clone()),
        &mut out,
    );
    if crate::commit_stamp().is_none() {
        assert_eq!(code, FAILED, "{out}");
        assert!(out.contains("GDFL IMPORT REFUSED"), "{out}");
        assert!(
            !pull::gdfl_import::journal_path(&store).exists(),
            "nothing was written"
        );
    } else {
        assert!(!out.contains("GDFL IMPORT REFUSED"), "{out}");
    }
    std::fs::remove_dir_all(root).unwrap();
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

/// **A WRITING VERB REFUSES A STORE WHOSE BARS FOLDER IS A LINK** (R3-16,
/// RA-298, FM-198), for the import, which takes its root without
/// `store_write::root_with`: a test shadow's `bars` linked to the real
/// store's is refused `linked_store_root` before a day is read, and nothing
/// is written, neither in the shadow nor through the link.
#[test]
fn gdfl_import_refuses_a_store_whose_bars_folder_is_a_link() {
    let root = scratch("linked-bars");
    let real = root.join("real");
    std::fs::create_dir_all(real.join("bars")).unwrap();
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    if std::fs::symlink_metadata(store.join("bars")).is_err() {
        std::os::unix::fs::symlink(real.join("bars"), store.join("bars")).unwrap();
    }
    std::fs::create_dir_all(root.join("ts")).unwrap();
    let env = env_of(&[(TICKSTORE_VAR, root.join("ts"))]);
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("linked_store_root"), "{out}");
    assert!(
        !pull::gdfl_import::journal_path(&store).exists(),
        "nothing was written"
    );
    assert_eq!(
        std::fs::read_dir(real.join("bars")).unwrap().count(),
        0,
        "nothing was written through the link"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// **FM-046: A DANGLING DAY-FILE LINK ENDS THE RUN FAILED, NEVER A DAY NOT
/// HELD** (R3-16), end to end through the verb. The tick store's day reader
/// opened with `File::open`, so a dangling link at a day's name (a year moved
/// to a drive that is unplugged) was `NotFound`, the day read as absent, and
/// the run closed `days not in source` with exit 0. Through the store's one
/// reading door it is refused `tick_store_malformed` naming the link, the
/// run exits FAILED, and the day is not reported as one the source does not
/// hold; the NIFTY options reader is refused the same way. A day with no
/// name at all is still "not in source", and since the GDFL import door
/// (AF-01, GD-01, GD-02) the tick store carries the `FORMAT.md` the door
/// asks of its root, and that run, which read no file while it owed the
/// day, exits FAILED `no_file_read` rather than OK.
#[test]
fn a_dangling_day_file_link_ends_the_run_failed_never_a_day_not_held() {
    let root = scratch("dangling-day");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let day = pull::session::Day::new(2024, 4, 1).unwrap();
    let tick_store = pull::gdfl_tickstore::TickStore::new(&ts);
    let options = pull::gdfl_nfo::NfoTickStore::new(&ts);
    for (kind, at) in [
        (
            "indices",
            tick_store.day_path(pull::gdfl_cm::CmKind::Indices, day),
        ),
        ("options", options.day_path(day)),
    ] {
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        if std::fs::symlink_metadata(&at).is_err() {
            std::os::unix::fs::symlink(root.join("an-unplugged-drive"), &at).unwrap();
        }
        let env = env_of(&[(TICKSTORE_VAR, ts.clone())]);
        let mut out = String::new();
        let code = command_with(
            &[kind, "tickstore", "2024-04-01", "2024-04-01", "NIFTY"],
            &env,
            &|| Ok(store.clone()),
            &mut out,
        );
        assert_eq!(code, FAILED, "{kind}: {out}");
        assert!(out.contains("symbolic link"), "{kind}: {out}");
        assert!(
            !out.contains("days not in source  1 2024-04-01"),
            "{kind}: a dangling link is not a day the source does not hold: {out}"
        );
        assert!(
            !root.join("an-unplugged-drive").exists(),
            "{kind}: nothing was made through the link"
        );
        std::fs::remove_file(&at).unwrap();
        let mut out = String::new();
        let code = command_with(
            &[kind, "tickstore", "2024-04-01", "2024-04-01", "NIFTY"],
            &env,
            &|| Ok(store.clone()),
            &mut out,
        );
        assert_eq!(code, FAILED, "{kind}: {out}");
        assert!(
            out.contains("GDFL IMPORT FAILED: no_file_read: "),
            "{kind}: {out}"
        );
        assert!(
            out.contains("days not in source  1 2024-04-01"),
            "{kind}: {out}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// The smallest zip there is: an end record and nothing else (22 bytes).
pub(super) fn empty_zip() -> Vec<u8> {
    let mut bytes = vec![0x50, 0x4b, 0x05, 0x06];
    bytes.extend_from_slice(&[0; 18]);
    bytes
}

/// A zip of STORED `members` (name, bytes), written the way the vendor's
/// outer archive is: local headers, the central directory, the end record.
/// The members here are invented; no vendor byte is in any of them.
pub(super) fn stored_zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    let u16le = |n: usize| u16::try_from(n).unwrap().to_le_bytes();
    let u32le = |n: usize| u32::try_from(n).unwrap().to_le_bytes();
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, bytes) in members {
        let crc = pull::gdfl_archive::crc32(bytes).to_le_bytes();
        let offset = out.len();
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&crc);
        out.extend_from_slice(&u32le(bytes.len()));
        out.extend_from_slice(&u32le(bytes.len()));
        out.extend_from_slice(&u16le(name.len()));
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(bytes);
        directory.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02, 20, 0, 20, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&[0, 0, 0, 0]);
        directory.extend_from_slice(&crc);
        directory.extend_from_slice(&u32le(bytes.len()));
        directory.extend_from_slice(&u32le(bytes.len()));
        directory.extend_from_slice(&u16le(name.len()));
        directory.extend_from_slice(&[0; 12]);
        directory.extend_from_slice(&u32le(offset));
        directory.extend_from_slice(name.as_bytes());
    }
    let at = out.len();
    out.extend_from_slice(&directory);
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
    out.extend_from_slice(&u16le(members.len()));
    out.extend_from_slice(&u16le(members.len()));
    out.extend_from_slice(&u32le(directory.len()));
    out.extend_from_slice(&u32le(at));
    out.extend_from_slice(&[0, 0]);
    out
}

/// A capital-market outer zip the door accepts for `kind`, holding one day
/// zip of that tree on 1 January 2019 whose own bytes are `inner`: none of
/// the days these tests ask for.
pub(super) fn cm_archive(kind: pull::gdfl_cm::CmKind, inner: &[u8]) -> Vec<u8> {
    let day = Day::new(2019, 1, 1).unwrap();
    let name = pull::gdfl_archive::day_zip_name(kind, day);
    stored_zip(&[(name.as_str(), inner)])
}

/// **A RUN THAT READ NO FILE NEVER EXITS OK (AF-01; FM-060, ingest-7).**
/// This test used to be `an_empty_source_reports_every_day_it_does_not_hold`,
/// and it pinned exit `OK` for all four pairs: a typo in the root's variable,
/// an empty staging folder or a vanished second volume read as success. Now
/// every pair whose root is what its source needs but holds none of the
/// range's days still lists the two days, and exits `FAILED` with
/// `no_file_read`; the existing-but-empty 22-byte capital-market zip, which
/// holds no day zip of either tree, is refused before any day as
/// `source_root_unknown`.
#[test]
fn a_run_that_read_no_file_never_exits_ok() {
    let root = scratch("empty");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let years = root.join("years");
    std::fs::create_dir_all(&years).unwrap();
    std::fs::write(years.join("2019.zip"), empty_zip()).unwrap();
    let cm = root.join("cm.zip");
    std::fs::write(&cm, cm_archive(pull::gdfl_cm::CmKind::Stocks, &empty_zip())).unwrap();
    let env = env_of(&[
        (TICKSTORE_VAR, ts),
        (CM_ARCHIVE_VAR, cm),
        (NFO_ARCHIVES_VAR, years),
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
        assert_eq!(code, FAILED, "{source} {kind}: {out}");
        assert!(
            out.contains("days not in source  2 2024-04-01 2024-04-02"),
            "{out}"
        );
        assert!(out.contains("1s only"), "{out}");
        assert!(
            out.contains("GDFL IMPORT FAILED: no_file_read: the run owed 2 days"),
            "{out}"
        );
    }
    // The 22-byte zip: refused before any day, the journal untouched.
    let before = std::fs::read(pull::gdfl_import::journal_path(&store)).unwrap();
    let empty = root.join("empty.zip");
    std::fs::write(&empty, empty_zip()).unwrap();
    let mut out = String::new();
    let code = command_with(
        &["stocks", "zip", "2024-04-01", "2024-04-02", "NIFTY"],
        &env_of(&[(CM_ARCHIVE_VAR, empty)]),
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("GDFL IMPORT REFUSED: source_root_unknown: BRUTEX_GDFL_CM_ARCHIVE=")
            && out.contains("holds no day zip of its tree"),
        "{out}"
    );
    assert_eq!(
        std::fs::read(pull::gdfl_import::journal_path(&store)).unwrap(),
        before
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_named_failure_or_a_refused_run_exits_failed() {
    let root = scratch("failed");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    tick_store(&root.join("ts"));
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
    // A range that runs backwards is refused before the lock as a misuse,
    // with the other garbage (AF-01, GD-09): exit MISUSED, never a run.
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-04-02", "2024-04-01"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(
        out.contains("refused: range_backwards: ") && out.contains("backwards"),
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
    tick_store(&ts);
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
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("GDFL IMPORT FAILED: no_file_read: "), "{out}");
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
            && run.contains(" format_sha256=")
            && !run.contains(" format_sha256=unmeasured"),
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
    // Moved with the scope into the one run authority (IA-1, D-4800).
    use pull::gdfl_run::scope::{
        of as scope_of, option_underlyings as stored_option_underlyings, options_tree,
        stored_filter,
    };
    let root = scratch("stored");
    let store = root.join("store");
    let tree = options_tree(&store);
    for name in ["HDFC", "NIFTY", "TATAMOTORS"] {
        std::fs::create_dir_all(tree.join(name)).unwrap();
    }
    // A file in the tree is not an underlying.
    std::fs::write(tree.join("README"), b"not a directory").unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
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
    // The stored scope is said, and a run that read no file is never OK.
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("GDFL IMPORT FAILED: no_file_read: "), "{out}");
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

/// WP-16: `BRUTEX_GDFL_WORKERS` is optional; when set it is a count from 1 to
/// the bound, and anything else is refused by name before anything is read.
#[test]
fn workers_are_the_default_or_a_count_within_the_bound() {
    let none = env_of(&[]);
    assert_eq!(workers_of(&none), Ok(Workers::for_host()));
    let four = env_of(&[(WORKERS_VAR, PathBuf::from("4"))]);
    assert_eq!(workers_of(&four).map(Workers::get), Ok(4));
    for bad in ["0", "13", "x", "-1", ""] {
        let env = env_of(&[(WORKERS_VAR, PathBuf::from(bad))]);
        let why = workers_of(&env).expect_err(bad);
        assert!(why.starts_with(WORKERS_VAR), "{bad}: {why}");
    }
    let store = scratch("workers");
    let env = env_of(&[
        (WORKERS_VAR, PathBuf::from("0")),
        (TICKSTORE_VAR, store.clone()),
    ]);
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.contains(WORKERS_VAR), "{out}");
    std::fs::remove_dir_all(&store).ok();
}

/// WP-16 (D-3353): `BRUTEX_GDFL_DURABILITY` is optional; when set it is one
/// of the three words, and anything else is refused by name before anything
/// is read. A run says which it used, and its workers.
#[test]
fn durability_is_the_default_or_one_of_its_words() {
    let none = env_of(&[]);
    assert_eq!(durability_of(&none), Ok(Durability::DEFAULT));
    for word in ["day", "day-each-file", "each-write"] {
        let env = env_of(&[(DURABILITY_VAR, PathBuf::from(word))]);
        assert_eq!(durability_of(&env).map(Durability::as_str), Ok(word));
    }
    for bad in ["", "Day", "daily", "each"] {
        let env = env_of(&[(DURABILITY_VAR, PathBuf::from(bad))]);
        let why = durability_of(&env).expect_err(bad);
        assert!(why.starts_with(DURABILITY_VAR), "{bad}: {why}");
    }
    let store = scratch("durability");
    let env = env_of(&[
        (DURABILITY_VAR, PathBuf::from("weekly")),
        (TICKSTORE_VAR, store.clone()),
    ]);
    let mut out = String::new();
    let code = command_with(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &env,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.contains(DURABILITY_VAR), "{out}");
    std::fs::remove_dir_all(&store).ok();
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

/// **NO RUN WORKS UNLOGGED** (IA-GR-44's cli half, D-4903): a process whose
/// sink was refused is refused `log_unavailable` after the misuse checks and
/// before any lock, root or scope: no `imports/` is made, no line is said.
#[test]
fn a_run_whose_sink_was_refused_refuses_before_any_lock() {
    let root = scratch("unlogged");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let env = env_of(&[(TICKSTORE_VAR, ts)]);
    let log = LogState::Refused("events.lock is held by pid 4242".to_owned());
    let said = Mutex::new(Vec::<String>::new());
    let say = |line: &str| said.lock().unwrap().push(line.to_owned());
    let mut out = String::new();
    let code = command_in(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        (&env, &|| Ok(store.clone())),
        (&log, &say),
        &mut out,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.starts_with("GDFL IMPORT REFUSED: log_unavailable: ")
            && out.contains("events.lock is held by pid 4242"),
        "{out}"
    );
    assert!(
        !store.join("imports").exists(),
        "no lock, no journal: {out}"
    );
    assert!(said.lock().unwrap().is_empty());
    // A misuse is still a misuse first, whatever the log.
    let mut out = String::new();
    let code = command_in(
        &["options", "tickstore", "2024-04-01", "2024-04-01", STORED],
        (&env, &|| Ok(store.clone())),
        (&log, &say),
        &mut out,
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.starts_with("refused: stored_with_options: "), "{out}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A run says a line a day through the one stderr writer it is handed, its
/// begin and its end, and leaves its progress record (D-4904) beside the
/// store's lock, finished, with the range's days in it.
#[test]
fn a_run_says_a_line_a_day_and_leaves_its_progress_record() {
    let root = scratch("told");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let env = env_of(&[(TICKSTORE_VAR, ts)]);
    let log = LogState::Installed(store.join("logs").join("gdfl").join("store"));
    let said = Mutex::new(Vec::<String>::new());
    let say = |line: &str| said.lock().unwrap().push(line.to_owned());
    let mut out = String::new();
    let _ = command_in(
        &[
            "indices",
            "tickstore",
            "2024-04-01",
            "2024-04-02",
            "--snapshot",
            "pre-1",
        ],
        (&env, &|| Ok(store.clone())),
        (&log, &say),
        &mut out,
    );
    let said = said.lock().unwrap().clone();
    assert_eq!(said.len(), 4, "{said:?}");
    assert!(said[0].starts_with("gdfl-import indices: run ") && said[0].ends_with(" begun"));
    assert!(
        said[1].starts_with("gdfl-import indices 2024-04-01: imported 0 done before 0 not held 1")
    );
    assert!(
        said[2].starts_with("gdfl-import indices 2024-04-02: imported 0 done before 0 not held 2")
    );
    assert!(said[3].ends_with(" finished"), "{said:?}");
    // No version-4 census here: the store lock's progress file.
    let progress =
        pull::gdfl_run::Progress::read(&store.join("imports").join("gdfl.progress")).unwrap();
    assert_eq!(progress.state, pull::gdfl_run::RunState::Finished.code());
    assert_eq!(
        (progress.layout, progress.day_total, progress.days_not_held),
        (3, 2, 2)
    );
    assert_eq!(progress.by, 1, "the cli");
    // A snapshot flag with no name after it is a misuse.
    let mut out = String::new();
    let code = command_in(
        &[
            "indices",
            "tickstore",
            "2024-04-01",
            "2024-04-02",
            "--snapshot",
        ],
        (&env, &|| Ok(store.clone())),
        (&log, &|_: &str| {}),
        &mut out,
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.contains("--snapshot takes a NAME"), "{out}");
    std::fs::remove_dir_all(root).unwrap();
}

/// Where a run verb logs (D-4862): any other verb, and a word that is not a
/// kind, keep `install_log`; a `BRUTEX_LOG_DIR` naming another directory is
/// refused without installing anything; the run's own directory installs.
#[test]
fn a_run_logs_to_its_own_directory_or_says_why_not() {
    let words = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
    assert_eq!(install_run_log(&words(&["sweep", "6", "100"])), None);
    assert_eq!(
        install_run_log(&words(&["gdfl-import", "futures", "zip"])),
        None
    );
    assert_eq!(install_run_log(&words(&["gdfl-import"])), None);
    let dir = Path::new("/srv/store/logs/gdfl/indices");
    let installs = std::cell::Cell::new(0);
    let install = |at: &Path| {
        assert_eq!(at, dir);
        installs.set(installs.get() + 1);
        Ok(())
    };
    let other = run_log_answer(
        Some(OsString::from("/srv/store/logs/cli-batch")),
        dir,
        &install,
    );
    assert!(
        other.starts_with(crate::UNLOGGED) && other.contains("Unset BRUTEX_LOG_DIR"),
        "{other}"
    );
    assert_eq!(installs.get(), 0, "nothing installed elsewhere");
    let own = run_log_answer(Some(dir.as_os_str().to_owned()), dir, &install);
    assert!(
        own.starts_with("events -> /srv/store/logs/gdfl/indices"),
        "{own}"
    );
    let none = run_log_answer(None, dir, &install);
    assert!(none.contains("D-4862"), "{none}");
    assert_eq!(installs.get(), 2);
    let refused = run_log_answer(None, dir, &|_: &Path| {
        Err("sink already installed".to_owned())
    });
    assert_eq!(
        refused,
        format!("{}: sink already installed", crate::UNLOGGED)
    );
}

/// **ONE DIRECTORY, TWO SPELLINGS** (IA-1, D-4862): the store root is
/// canonical, so a `BRUTEX_LOG_DIR` naming the run's own directory through a
/// link (as `/var` names `/private/var` on macOS) is the run's directory,
/// also before it exists; a link to another tree, a `..` out of it and a
/// sibling kind's directory are refused by name and install nothing.
#[test]
fn a_log_directory_spelled_through_a_link_is_the_runs_own() {
    let root = scratch(&format!("log-spelling-{}", std::process::id()));
    let (real, other) = (root.join("real"), root.join("other"));
    std::fs::create_dir_all(&real).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let (link, other_link) = (root.join("link"), root.join("other-link"));
    if !link.exists() {
        std::os::unix::fs::symlink(&real, &link).unwrap();
    }
    if !other_link.exists() {
        std::os::unix::fs::symlink(&other, &other_link).unwrap();
    }
    let kind = |base: &Path, name: &str| base.join("logs").join("gdfl").join(name);
    let dir = kind(&std::fs::canonicalize(&real).unwrap(), "indices");
    assert!(!dir.exists(), "not made before the run");
    let installs = std::cell::Cell::new(0);
    let install = |at: &Path| {
        assert_eq!(at, dir);
        installs.set(installs.get() + 1);
        Ok(())
    };
    let through = kind(&link, "indices");
    let said = run_log_answer(Some(through.into_os_string()), &dir, &install);
    assert!(said.starts_with("events -> "), "{said}");
    assert_eq!(installs.get(), 1);
    let refused = [
        kind(&other_link, "indices"),
        kind(&link, "indices").join("..").join("stocks"),
        kind(&real, "stocks"),
    ];
    for named in refused {
        let said = run_log_answer(Some(named.clone().into_os_string()), &dir, &install);
        assert!(
            said.starts_with(crate::UNLOGGED) && said.contains("Unset BRUTEX_LOG_DIR"),
            "{}: {said}",
            named.display()
        );
    }
    assert_eq!(installs.get(), 1, "nothing installed elsewhere");
}

/// **EVERY FAILURE COUNTED, THE NEWEST LISTED** (D-4839): a receipt of more
/// failures than a report keeps says the whole count, how many are not
/// listed and each stage and code's count.
#[test]
fn the_receipt_says_how_many_failures_it_does_not_list() {
    let mut report = Report::default();
    let cause = pull::gdfl_import::Cause::open(pull::gdfl_import::Stage::Vendor, "malformed_row");
    for i in 0..1_100 {
        report.name(
            cause,
            pull::ingest::Failure {
                instrument: format!("F{i}"),
                why: "invented".to_owned(),
                month: None,
                day: None,
            },
        );
    }
    report.seal();
    let mut out = String::new();
    render(
        &mut out,
        ImportKind::Options,
        "zip",
        Path::new("/s"),
        &report,
    );
    assert!(out.contains("  failures            1100\n"), "{out}");
    assert!(
        out.contains("(76 more not listed; the newest 1024 are;"),
        "{out}"
    );
    assert!(out.contains("    vendor/malformed_row: 1100\n"), "{out}");
    assert!(
        out.contains("    F1099: invented") && !out.contains("    F75: "),
        "{out}"
    );
}
