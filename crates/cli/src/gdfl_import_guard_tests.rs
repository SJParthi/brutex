#![cfg(test)]
//! Tests of the `gdfl-import` door (AF-01, D-4977). Every store, tick store
//! and archive here is synthetic: invented words in `FORMAT.md`, empty or
//! one-member zips, journal lines written by hand. No vendor byte.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt as _;
use std::time::{Duration, Instant};

use pull::gdfl_import::journal_path;
use pull::session::Day;

use super::super::tests::{cm_archive, empty_zip, env_of, scratch, stored_zip, tick_store};
use super::super::{CM_ARCHIVE_VAR, NFO_ARCHIVES_VAR, TICKSTORE_VAR, command_with};
use super::*;
use crate::{FAILED, MISUSED, OK};

/// The verb on `store`, its words and its exit.
fn run(args: &[&str], env: &impl Fn(&str) -> Option<OsString>, store: &Path) -> (u8, String) {
    let mut out = String::new();
    let code = command_with(args, env, &|| Ok(store.to_path_buf()), &mut out);
    (code, out)
}

/// A holder this test names, never this process.
fn holder(kind: ImportKind) -> Holder {
    Holder {
        pid: 4242,
        run_started: 1_700_000_000,
        kind,
        verb: verb_of(&["options", "zip", "2024-04-01", "2024-04-01"]),
    }
}

/// A directory of yearly options zips holding one, for 2019.
fn years(root: &Path) -> PathBuf {
    let dir = root.join("years");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("2019.zip"), empty_zip()).unwrap();
    dir
}

/// A store whose census is partitioned (D-3316's v4): each kind its lock.
fn v4_store(root: &Path) -> PathBuf {
    let store = root.join("store");
    fs::create_dir_all(store.join("manifest").join("v4").join("gdfl")).unwrap();
    store
}

/// Whether this host enforces a directory's write bit on this user (root
/// does not), probed on `dir` set read-only.
fn modes_bind(dir: &Path) -> bool {
    let probe = dir.join("probe");
    let bound = fs::create_dir(&probe).is_err();
    if !bound {
        fs::remove_dir(&probe).unwrap();
    }
    bound
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// What a run's report says when every owed day was missing from its source
/// and nothing was read.
fn nothing_read(days: &[&str]) -> Report {
    Report {
        days_missing: days.iter().map(|d| super::super::day(d).unwrap()).collect(),
        ..Report::default()
    }
}

/// The two done days of the change tests.
const TWO: [&str; 2] = ["2024-04-01", "2024-04-02"];

/// A capital-market outer zip holding a day zip of `kind` on each of `days`,
/// each one's bytes `inner` (invented). Since WP-12f (D-4878) a done day
/// whose close did not cover the run's whole scope has its listing probed,
/// and a source that no longer holds the day fails the run; these tests
/// therefore hold their done days, with nothing in them to add.
fn cm_days(kind: CmKind, days: &[&str], inner: &[u8]) -> Vec<u8> {
    let names: Vec<String> = days
        .iter()
        .map(|d| day_zip_name(kind, super::super::day(d).unwrap()))
        .collect();
    let members: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), inner)).collect();
    stored_zip(&members)
}

// ═══ 1. The run lock ════════════════════════════════════════════════════════

/// **A SECOND RUN OF A KIND IS REFUSED BEFORE ANY DAY, NAMING THE HOLDER
/// (FM-064, ingest-4).** Two runs of one kind used to begin the same day; the
/// second now stops at the door with `run_busy`, the holder's pid and start,
/// and the journal is not written. Once released, the kind runs again.
#[test]
fn a_second_run_of_the_same_kind_is_refused_naming_the_holder() {
    let root = scratch("door-busy");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let env = env_of(&[(NFO_ARCHIVES_VAR, years(&root))]);
    let held = RunLock::take(&store, &holder(ImportKind::Options)).unwrap();
    assert_eq!(held.name, "imports/gdfl.lock", "a v3 store has one lock");
    let (code, out) = run(
        &["options", "zip", "2024-04-01", "2024-04-01"],
        &env,
        &store,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("GDFL IMPORT REFUSED: run_busy: another options run holds imports/gdfl.lock: pid 4242 started 1700000000"),
        "{out}"
    );
    assert!(!journal_path(&store).exists(), "refused before any day");
    held.release().unwrap();
    let (code, out) = run(
        &["options", "zip", "2024-04-01", "2024-04-01"],
        &env,
        &store,
    );
    assert!(!out.contains(RUN_BUSY), "{out}");
    assert_eq!(code, FAILED, "nothing was read: {out}");
    assert_eq!(
        fs::read(store.join("imports/gdfl.lock")).unwrap(),
        b"",
        "a clean end clears its record"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **ANOTHER KIND RUNS BESIDE IT ON V4.** The v4 census runs kinds in
/// parallel by design (`run_dirs`'s table): each kind takes its own lock. On v3
/// one lock serialises every kind.
#[test]
fn a_run_of_another_kind_runs_beside_it() {
    let root = scratch("door-beside");
    let store = v4_store(&root);
    let ts = root.join("ts");
    tick_store(&ts);
    let options = RunLock::take(&store, &holder(ImportKind::Options)).unwrap();
    assert_eq!(options.name, "imports/gdfl.options.lock");
    let indices = RunLock::take(&store, &holder(ImportKind::Indices)).unwrap();
    assert_eq!(indices.name, "imports/gdfl.indices.lock");
    let again = RunLock::take(&store, &holder(ImportKind::Options)).unwrap_err();
    assert_eq!(again.code, RUN_BUSY, "{again}");
    indices.release().unwrap();
    let (_, out) = run(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &env_of(&[(TICKSTORE_VAR, ts)]),
        &store,
    );
    assert!(!out.contains(RUN_BUSY), "{out}");
    assert!(
        out.contains("run lock            imports/gdfl.indices.lock (v4; pid "),
        "{out}"
    );
    options.release().unwrap();

    let v3 = root.join("v3");
    fs::create_dir_all(&v3).unwrap();
    let options = RunLock::take(&v3, &holder(ImportKind::Options)).unwrap();
    let indices = RunLock::take(&v3, &holder(ImportKind::Indices)).unwrap_err();
    assert_eq!(indices.code, RUN_BUSY, "{indices}");
    assert!(
        indices
            .words
            .starts_with("another options run holds imports/gdfl.lock: pid 4242"),
        "the holder's own kind, from its record, not the asking run's: {indices}"
    );
    options.release().unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// The child half of [`a_killed_holders_lock_is_freed_and_its_death_named`]:
/// with `AF01_HOLD_STORE` set it takes the options lock of that store, says
/// so, and waits to be killed. Alone it takes and releases a lock of its own.
#[test]
#[ignore = "the child process of a_killed_holders_lock_is_freed_and_its_death_named"]
fn holder_child() {
    if let Some(store) = std::env::var_os("AF01_HOLD_STORE") {
        let lock = RunLock::take(Path::new(&store), &holder(ImportKind::Options)).unwrap();
        assert!(lock.dead.is_none());
        println!("AF01 HELD");
        std::thread::sleep(Duration::from_mins(2));
        lock.release().unwrap();
    } else {
        let root = scratch("door-child");
        RunLock::take(&root, &holder(ImportKind::Options))
            .unwrap()
            .release()
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}

/// **A KILLED HOLDER IS FREED BY THE KERNEL AND NAMED AS DEAD.** A real
/// process takes the lock and is killed with SIGKILL; while it lives the kind
/// is busy, and after it the next run takes the lock and says the previous
/// holder died (its record present, its lock free).
#[test]
fn a_killed_holders_lock_is_freed_and_its_death_named() {
    let root = scratch("door-killed");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "gdfl_import_command::guard::tests::holder_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("AF01_HOLD_STORE", &store)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let lock = store.join("imports/gdfl.lock");
    let deadline = Instant::now() + Duration::from_mins(1);
    while fs::read(&lock).map_or(true, |b| b.is_empty()) {
        assert!(Instant::now() < deadline, "the child never took the lock");
        std::thread::sleep(Duration::from_millis(20));
    }
    let busy = RunLock::take(&store, &holder(ImportKind::Options)).unwrap_err();
    assert_eq!(busy.code, RUN_BUSY, "{busy}");
    child.kill().unwrap();
    child.wait().unwrap();
    let taken = RunLock::take(&store, &holder(ImportKind::Options)).unwrap();
    let dead = taken.dead.clone().expect("the record stayed");
    assert!(
        dead.contains("pid=4242") && dead.starts_with(HOLDER_V1),
        "{dead}"
    );
    taken.release().unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// **A DEAD HOLDER'S LOCK IS TAKEN AND ITS DEATH NAMED ON THE RECEIPT.** The
/// record a killed holder leaves (the lock free, the record present) is
/// printed, and this run's clean end clears it.
#[test]
fn a_dead_holders_lock_is_taken_and_its_death_named() {
    let root = scratch("door-dead");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let left = holder(ImportKind::Options).record(GdflLayout::V3, 1);
    fs::write(store.join("imports/gdfl.lock"), &left).unwrap();
    let env = env_of(&[(NFO_ARCHIVES_VAR, years(&root))]);
    let (_, out) = run(
        &["options", "zip", "2024-04-06", "2024-04-07"],
        &env,
        &store,
    );
    assert!(
        out.contains(
            "previous holder     died: pid 4242 started 1700000000 (record present, lock free)"
        ),
        "{out}"
    );
    assert_eq!(fs::read(store.join("imports/gdfl.lock")).unwrap(), b"");
    fs::remove_dir_all(root).unwrap();
}

/// **A LINK OR A DIRECTORY AT THE LOCK'S NAME IS REFUSED BY NAME.** The link
/// is never followed (its target is untouched); nothing is journalled.
#[test]
fn a_lock_path_that_is_a_link_or_a_directory_is_refused_by_name() {
    let root = scratch("door-lockpath");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let target = root.join("elsewhere");
    fs::write(&target, b"not a lock").unwrap();
    std::os::unix::fs::symlink(&target, store.join("imports/gdfl.lock")).unwrap();
    let env = env_of(&[(NFO_ARCHIVES_VAR, years(&root))]);
    let (code, out) = run(
        &["options", "zip", "2024-04-01", "2024-04-01"],
        &env,
        &store,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("run_lock_refused") && out.contains("a symbolic link (never followed)"),
        "{out}"
    );
    assert_eq!(fs::read(&target).unwrap(), b"not a lock");
    assert!(!journal_path(&store).exists());
    fs::remove_file(store.join("imports/gdfl.lock")).unwrap();
    fs::create_dir(store.join("imports/gdfl.lock")).unwrap();
    let refused = RunLock::take(&store, &holder(ImportKind::Options)).unwrap_err();
    assert_eq!(refused.code, RUN_LOCK_REFUSED);
    assert!(refused.words.contains("a directory"), "{refused}");
    fs::remove_dir_all(root).unwrap();
}

/// **A HARD LINK AT THE LOCK'S NAME IS REFUSED, AND ITS FILE LEFT WHOLE
/// (X1).** The lock was opened `O_NOFOLLOW`, which a hard link passes: the
/// record's `set_len(0)` and write went through it and emptied whatever file
/// the other name was. Now a lock file with more than one name is refused
/// `run_lock_refused` before a byte is written, and nothing is journalled.
#[test]
fn a_hard_link_at_the_lock_name_is_refused_and_its_file_left_whole() {
    let root = scratch("door-hardlink");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let decoy = root.join("decoy");
    fs::write(&decoy, b"INVENTED-DECOY-BYTES").unwrap();
    fs::hard_link(&decoy, store.join("imports/gdfl.lock")).unwrap();
    let env = env_of(&[(NFO_ARCHIVES_VAR, years(&root))]);
    let (code, out) = run(
        &["options", "zip", "2024-04-01", "2024-04-01"],
        &env,
        &store,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("GDFL IMPORT REFUSED: run_lock_refused: ")
            && out.contains("has 2 names (a hard link to another file)"),
        "{out}"
    );
    assert_eq!(fs::read(&decoy).unwrap(), b"INVENTED-DECOY-BYTES");
    assert!(!journal_path(&store).exists());
    assert!(!sources_path(&store).exists());
    // The other name gone, the lock file is a lock file again.
    fs::remove_file(&decoy).unwrap();
    let lock = RunLock::take(&store, &holder(ImportKind::Options)).unwrap();
    lock.release().unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// **A HARD LINK AT THE SOURCES LEDGER'S NAME IS REFUSED, AND ITS FILE LEFT
/// WHOLE (GD-10).** The ledger is opened `O_NOFOLLOW` to append, which a
/// hard link passes, so the run's line would land in whatever file the other
/// name is. A ledger file with more than one name is now refused
/// `sources_ledger_unwritten`, the run ends `FAILED` and the other file keeps
/// every byte.
#[test]
fn a_hard_link_at_the_ledger_name_is_refused_and_its_file_left_whole() {
    let root = scratch("door-ledger-hardlink");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let decoy = root.join("decoy");
    fs::write(&decoy, b"INVENTED-DECOY-BYTES\n").unwrap();
    fs::hard_link(&decoy, sources_path(&store)).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let (code, out) = run(
        &["indices", "tickstore", "2024-04-06", "2024-04-07"],
        &env_of(&[(TICKSTORE_VAR, ts)]),
        &store,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("GDFL IMPORT FAILED: sources_ledger_unwritten: ")
            && out.contains("has 2 names (a hard link to another file)"),
        "{out}"
    );
    assert_eq!(fs::read(&decoy).unwrap(), b"INVENTED-DECOY-BYTES\n");
    // The lock was released all the same: the kind runs again.
    let lock = RunLock::take(&store, &holder(ImportKind::Indices)).unwrap();
    lock.release().unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// **A HOLDER RECORD OF FOREIGN BYTES IS PRINTED ESCAPED (X5).** Whatever is
/// left at the lock's name (random bytes, a terminal control sequence, a
/// newline followed by words that look like a receipt line) is printed as
/// one escaped line: it can forge no line and move no cursor. A path that
/// holds a newline is escaped the same way.
#[test]
fn a_holder_record_of_foreign_bytes_is_printed_escaped() {
    let root = scratch("door-foreign");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    fs::write(
        store.join("imports/gdfl.lock"),
        b"\x1b[2J\nGDFL IMPORT OK\tpid=7 \xff\n",
    )
    .unwrap();
    let env = env_of(&[(NFO_ARCHIVES_VAR, years(&root))]);
    let (_, out) = run(
        &["options", "zip", "2024-04-06", "2024-04-07"],
        &env,
        &store,
    );
    assert!(
        out.contains("(record present, lock free): \\x1b[2J\\nGDFL IMPORT OK\\tpid=7 \\xff"),
        "{out}"
    );
    assert!(!out.contains('\x1b'), "{out:?}");
    assert!(
        !out.lines().any(|l| l.starts_with("GDFL IMPORT OK")),
        "{out}"
    );
    assert_eq!(fs::read(store.join("imports/gdfl.lock")).unwrap(), b"");
    assert_eq!(shown(Path::new("/a\nb\tc")), "/a\\nb\\tc");
    fs::remove_dir_all(root).unwrap();
}

/// **THE LOCK NEVER MAKES A STORE ROOT THAT IS GONE.** `imports/` alone is
/// made: a store root whose volume went is `store_lost`, and is not born
/// again empty on a writable parent (it was, by `create_dir_all`).
#[test]
fn the_lock_never_makes_a_store_root_that_is_gone() {
    let root = scratch("door-noroot");
    let store = root.join("gone-store");
    let refused = RunLock::take(&store, &holder(ImportKind::Indices)).unwrap_err();
    assert_eq!(refused.code, STORE_LOST, "{refused}");
    assert!(!store.exists(), "the store root was made: {refused}");
    fs::remove_dir_all(root).unwrap();
}

/// **GARBAGE KIND OR DATES, AND A BACKWARDS RANGE, ARE REFUSED BEFORE THE
/// LOCK.** No lock file and no journal exist afterwards.
#[test]
fn garbage_kind_or_dates_are_refused_before_the_lock() {
    let root = scratch("door-garbage");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let env = env_of(&[(TICKSTORE_VAR, ts)]);
    for args in [
        ["futures", "tickstore", "2024-04-01", "2024-04-01"],
        ["indices", "tickstore", "2024-13-01", "2024-04-01"],
        ["indices", "tickstore", "2024-04-01", "\u{0}"],
        ["indices", "ftp", "2024-04-01", "2024-04-01"],
    ] {
        let (code, out) = run(&args, &env, &store);
        assert_eq!(code, MISUSED, "{args:?}: {out}");
    }
    // A range that runs backwards is a misuse like the others (exit 2, not
    // 1), and so is an empty SYMBOL word, which used to be read as a filter
    // matching no file: each before the lock, with nothing written.
    let (code, out) = run(
        &["indices", "tickstore", "2024-04-02", "2024-04-01"],
        &env,
        &store,
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(
        out.contains(
            "refused: range_backwards: the range runs backwards: 2024-04-02 is after 2024-04-01"
        ),
        "{out}"
    );
    for symbols in [&[""][..], &["NIFTY", " "][..]] {
        let mut args = vec!["indices", "tickstore", "2024-04-01", "2024-04-01"];
        args.extend_from_slice(symbols);
        let (code, out) = run(&args, &env, &store);
        assert_eq!(code, MISUSED, "{symbols:?}: {out}");
        assert!(
            out.contains("refused: symbol_empty: an empty SYMBOL names no instrument"),
            "{out}"
        );
    }
    assert!(!store.join("imports").exists(), "no lock, no journal");
    fs::remove_dir_all(root).unwrap();
}

// ═══ 2. The source root ═════════════════════════════════════════════════════

/// **A WRONG SOURCE ROOT IS REFUSED BEFORE ANY DAY IS JOURNALLED (FM-060,
/// ingest-7).** A tick store with no `FORMAT.md`, an options folder with no
/// `<yyyy>.zip`, a capital-market zip holding no day zip of its tree (and a
/// missing one, and a folder where a zip belongs): each refused
/// `source_root_unknown`, naming the variable, the path and what was
/// expected; an existing journal is byte-identical afterwards.
#[test]
fn a_wrong_source_root_is_refused_before_any_day_is_journalled() {
    let root = scratch("door-wrong");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let journal = journal_path(&store);
    fs::write(&journal, b"run 1700000000 source=zip\n").unwrap();
    let bare = root.join("bare");
    fs::create_dir_all(&bare).unwrap();
    let folder = root.join("folder");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("notes.txt"), b"x").unwrap();
    fs::write(folder.join("19.zip"), empty_zip()).unwrap();
    fs::create_dir_all(folder.join("2019.zip")).unwrap();
    let empty = root.join("empty.zip");
    fs::write(&empty, empty_zip()).unwrap();
    let indices_only = root.join("indices.zip");
    fs::write(&indices_only, cm_archive(CmKind::Indices, &empty_zip())).unwrap();
    let cases: [(&str, &str, &str, PathBuf, &str); 7] = [
        (
            "indices",
            "tickstore",
            TICKSTORE_VAR,
            bare.clone(),
            "holds no FORMAT.md at its root",
        ),
        (
            "options",
            "tickstore",
            TICKSTORE_VAR,
            root.join("typo"),
            "does not exist",
        ),
        (
            "options",
            "zip",
            NFO_ARCHIVES_VAR,
            folder.clone(),
            "holds no <yyyy>.zip",
        ),
        (
            "stocks",
            "zip",
            CM_ARCHIVE_VAR,
            empty,
            "holds no day zip of its tree",
        ),
        (
            "stocks",
            "zip",
            CM_ARCHIVE_VAR,
            indices_only,
            "holds no day zip of its tree",
        ),
        (
            "indices",
            "zip",
            CM_ARCHIVE_VAR,
            root.join("absent.zip"),
            "does not exist",
        ),
        ("indices", "zip", CM_ARCHIVE_VAR, bare, "is not a file"),
    ];
    for (kind, source, var, path, found) in cases {
        let (code, out) = run(
            &[kind, source, "2024-04-01", "2024-04-02"],
            &env_of(&[(var, path.clone())]),
            &store,
        );
        assert_eq!(code, FAILED, "{out}");
        let want = format!(
            "GDFL IMPORT REFUSED: source_root_unknown: {var}={} {found}",
            path.display()
        );
        assert!(
            out.contains(&want) && out.contains("; expected "),
            "{out} lacks {want}"
        );
        assert_eq!(fs::read(&journal).unwrap(), b"run 1700000000 source=zip\n");
        assert!(
            !store.join("imports/gdfl.lock").exists(),
            "refused before the lock"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

/// The origin's own fault reaches the refusal (it used to reach only the
/// "run started" event).
#[test]
fn the_origins_fault_is_printed_with_the_refusal() {
    let root = scratch("door-fault");
    let origin = Origin::measure(SourceKind::Zip, &root.join("absent.zip"), None);
    assert!(origin.fault.is_some());
    let refused = source_root(
        Shape::CmArchive(CmKind::Stocks),
        CM_ARCHIVE_VAR,
        &origin.path,
        &origin,
    )
    .unwrap_err();
    assert!(
        refused.words.contains("the origin measured: ")
            && refused.words.contains(origin.fault.as_deref().unwrap()),
        "{refused}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// What a source or store root may be named to forge a receipt line: a
/// newline, the door's own success-looking words, and a terminal sequence
/// that clears the screen (verify-AF-01-r2 blocking item 2).
const FORGED: &str = "bad\nGDFL IMPORT OK forged\x1b[2J";

/// What [`FORGED`] prints as.
const FORGED_SHOWN: &str = "bad\\nGDFL IMPORT OK forged\\x1b[2J";

/// The faults of `out` as a receipt: a raw control byte other than the
/// newline that ends each line, or a line that does not begin with the
/// door's own words.
fn forgeries(out: &str) -> Vec<String> {
    let mut faults: Vec<String> = out
        .bytes()
        .filter(|&b| (b < 0x20 && b != b'\n') || b == 0x7f)
        .map(|b| format!("raw control byte 0x{b:02x}"))
        .collect();
    let own = [
        "GDFL IMPORT -- ",
        "GDFL IMPORT REFUSED: ",
        "GDFL IMPORT FAILED: ",
        "refused: ",
        "  ",
    ];
    faults.extend(
        out.lines()
            .filter(|line| !own.iter().any(|word| line.starts_with(word)))
            .map(|line| format!("a line not the door's: {line:?}")),
    );
    faults
}

/// **NOTHING A ROOT'S NAME CARRIES FORGES A RECEIPT LINE (GD-11).** The door
/// printed the origin's fault unescaped, and the fault carries the source
/// root through `display()`: a root named [`FORGED`] printed a line of its own
/// beginning `GDFL IMPORT OK forged` and a raw clear-screen sequence. Every
/// source shape is run with a root of that name, absent (refused, with the
/// origin's fault) and present (run to its end), and with a store root of
/// that name: each receipt has exactly the lines the same run has under a
/// plain name, no raw control byte, no line that is not the door's, and the
/// name shown escaped.
#[test]
fn nothing_a_roots_name_carries_forges_a_receipt_line() {
    let root = scratch("door-forged");
    let receipt = |tag: &str, name: &str, present: bool, store_named: bool| {
        let base = root.join(tag);
        fs::create_dir_all(&base).unwrap();
        let mut rows = Vec::new();
        for (kind, source, var) in [
            ("indices", "tickstore", TICKSTORE_VAR),
            ("options", "zip", NFO_ARCHIVES_VAR),
            ("stocks", "zip", CM_ARCHIVE_VAR),
        ] {
            let dir = base.join(format!("{kind}-{source}"));
            fs::create_dir_all(&dir).unwrap();
            let data = dir.join(name);
            if present {
                match var {
                    TICKSTORE_VAR => tick_store(&data),
                    NFO_ARCHIVES_VAR => {
                        fs::create_dir_all(&data).unwrap();
                        fs::write(data.join("2024.zip"), empty_zip()).unwrap();
                    }
                    _ => fs::write(&data, cm_archive(CmKind::Stocks, &empty_zip())).unwrap(),
                }
            }
            let store = if store_named {
                dir.join(format!("store-{name}"))
            } else {
                dir.join("store")
            };
            fs::create_dir_all(&store).unwrap();
            let (code, out) = run(
                &[kind, source, "2024-04-01", "2024-04-01"],
                &env_of(&[(var, data)]),
                &store,
            );
            rows.push((code, out));
        }
        rows
    };
    for (present, store_named) in [(false, false), (true, false), (true, true)] {
        let plain_rows = receipt(
            &format!("plain-{present}-{store_named}"),
            "bad-plain",
            present,
            store_named,
        );
        let forged_rows = receipt(
            &format!("forged-{present}-{store_named}"),
            FORGED,
            present,
            store_named,
        );
        for ((plain_code, plain_out), (code, out)) in plain_rows.iter().zip(&forged_rows) {
            assert_eq!(code, plain_code, "{out}");
            assert_eq!(*code, FAILED, "{out}");
            assert_eq!(forgeries(out), Vec::<String>::new(), "{out}");
            assert_eq!(forgeries(plain_out), Vec::<String>::new(), "{plain_out}");
            assert_eq!(
                out.lines().count(),
                plain_out.lines().count(),
                "the forged name added or removed a line:\n{out}\n---\n{plain_out}"
            );
            assert!(
                !out.lines().any(|line| line.starts_with("GDFL IMPORT OK")),
                "{out}"
            );
            // A refusal names the source root; a receipt names the store root.
            if !present || store_named {
                assert!(
                    out.contains(FORGED_SHOWN),
                    "the name is shown escaped: {out}"
                );
            }
        }
    }
    fs::remove_dir_all(root).unwrap();
}

/// **THE CENSUS'S REFUSAL NAMING A FORGED STORE ROOT FORGES NOTHING (GD-11).**
/// [`layout_of`] passes on `census_store::locate`'s words, which carry the
/// store root through `display()` (verify-AF-01-r2 blocking item 2): a store
/// root named [`FORGED`] whose `manifest/v4/gdfl` is a file is refused, by
/// the run lock's code, before any day, with the root shown escaped, no raw
/// control byte and no line that is not the door's.
#[test]
fn the_census_refusal_naming_a_forged_store_root_forges_nothing() {
    let root = scratch("door-forged-census");
    let source = root.join("ts");
    tick_store(&source);
    let store = root.join(FORGED);
    fs::create_dir_all(store.join("manifest").join("v4")).unwrap();
    fs::write(
        store.join("manifest").join("v4").join("gdfl"),
        b"not a directory",
    )
    .unwrap();
    let (code, out) = run(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &env_of(&[(TICKSTORE_VAR, source)]),
        &store,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains(RUN_LOCK_REFUSED), "{out}");
    assert!(
        out.contains("the census layout that decides the run lock cannot be read"),
        "{out}"
    );
    assert!(
        out.contains(FORGED_SHOWN),
        "the root is shown escaped: {out}"
    );
    assert_eq!(forgeries(&out), Vec::<String>::new(), "{out}");
    assert!(
        !store.join("imports").exists(),
        "nothing was written into the store before the refusal"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **WORDS FROM ELSEWHERE ARE ONE LINE, AND STAY READABLE.** [`plain`]
/// escapes every control character (C0, DEL, C1), the bidirectional
/// controls and the line and paragraph separators, keeps every other
/// character (`\` and non-ASCII text included), and is idempotent, so a
/// [`shown`] path inside it passes unchanged. [`Finding::new`] applies it, so
/// no refusal of the door (the census's words in [`layout_of`] included)
/// carries a raw control byte.
#[test]
fn words_from_elsewhere_are_printed_plain() {
    let raw = "a\nb\tc\rd\x1b[2J\x07\x7f\u{85}\u{202e}\u{2028}\u{2066}\u{61c}e — é \\x";
    let want =
        "a\\nb\\tc\\rd\\x1b[2J\\x07\\x7f\\u{0085}\\u{202e}\\u{2028}\\u{2066}\\u{061c}e — é \\x";
    assert_eq!(plain(raw), want);
    assert_eq!(plain(want), want, "idempotent");
    let path = Path::new("/tmp").join(FORGED);
    assert_eq!(plain(&shown(&path)), shown(&path));
    assert_eq!(plain(""), "");
    let finding = Finding::new(RUN_LOCK_REFUSED, format!("census: {}", path.display()));
    assert_eq!(finding.words, format!("census: /tmp/{FORGED_SHOWN}"));
    assert!(!finding.to_string().contains(['\n', '\x1b']));
}

/// **A TORN LINE LONGER THAN THE WINDOW IS NAMED AND ENDED.** A ledger whose
/// last [`TAIL_WINDOW`] bytes hold no newline (a killed append of a line
/// that long, or bytes that were never a ledger) was read as a clean, empty
/// tail: nothing was named and the next run's line was glued onto the torn
/// bytes and lost (verify-AF-01-r2 follow-up). The whole window is the torn
/// line, and the append starts a line of its own after it.
#[test]
fn a_torn_line_longer_than_the_window_is_named_and_ended() {
    let root = scratch("door-long-torn");
    let ledger = root.join(SOURCES);
    let mut text = entry(&hex(&[1; 32]), 1).line();
    text.push_str(&"x".repeat(usize::try_from(TAIL_WINDOW).unwrap() + 10));
    fs::write(&ledger, &text).unwrap();
    let tail = tail_for(&ledger, KEY).unwrap();
    assert!(tail.cut);
    assert_eq!(tail.torn, usize::try_from(TAIL_WINDOW).unwrap());
    assert!(tail.last.is_none() && tail.unreadable == 0);
    let line = entry(&hex(&[2; 32]), 2).line();
    append(&ledger, &line).unwrap();
    let after = tail_for(&ledger, KEY).unwrap();
    assert_eq!(after.torn, 0);
    assert_eq!(
        after.last,
        Some(entry(&hex(&[2; 32]), 2)),
        "the line is whole"
    );
    assert!(
        fs::read_to_string(&ledger)
            .unwrap()
            .ends_with(&format!("x\n{line}"))
    );
    fs::remove_dir_all(root).unwrap();
}

/// **WHETHER THE LEDGER ENDS TORN IS DECIDED AT THE APPEND.** The decision
/// was made when the door opened and used when it closed; a run of another
/// kind (v4 runs kinds side by side) or a killed append can tear the end in
/// between, and the run's line was then glued onto the torn bytes. It is
/// read from the file's last byte at the append; an empty ledger and one
/// that ends a line get no extra newline.
#[test]
fn whether_the_ledger_ends_torn_is_decided_at_the_append() {
    let root = scratch("door-torn-late");
    let ledger = root.join(SOURCES);
    let first = entry(&hex(&[1; 32]), 1).line();
    append(&ledger, &first).unwrap();
    assert_eq!(
        fs::read_to_string(&ledger).unwrap(),
        first,
        "no newline before the first line"
    );
    let opened = tail_for(&ledger, KEY).unwrap();
    assert_eq!(opened.torn, 0, "clean when the door opened");
    // Another kind's append is killed mid-line after this door opened.
    let mut file = OpenOptions::new().append(true).open(&ledger).unwrap();
    io::Write::write_all(&mut file, b"v1\t9\topti").unwrap();
    drop(file);
    let second = entry(&hex(&[2; 32]), 2).line();
    append(&ledger, &second).unwrap();
    let text = fs::read_to_string(&ledger).unwrap();
    assert_eq!(text, format!("{first}v1\t9\topti\n{second}"));
    assert_eq!(
        tail_for(&ledger, KEY).unwrap().last,
        Some(entry(&hex(&[2; 32]), 2))
    );
    let third = entry(&hex(&[3; 32]), 3).line();
    append(&ledger, &third).unwrap();
    assert_eq!(
        fs::read_to_string(&ledger).unwrap(),
        format!("{first}v1\t9\topti\n{second}{third}"),
        "a ledger that ends a line gets no extra newline"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **A RUN THAT READ NO FILE IS NOT ACCEPTED BY RUNNING IT AGAIN.** The
/// importer closes a held day whose files the filter did not want as done
/// with no file; the identical next run skipped it, owed nothing and exited
/// `OK` with ledger outcome `ok` (verify-AF-01-r2 follow-up, TATAMOTORS on
/// 2019-06-03 on the real tick store). It is `FAILED` `no_file_read` again,
/// and stays so on every rerun until a run of the key reads a file; a run of
/// another filter is another key and is not named.
#[test]
fn a_run_that_read_no_file_is_not_accepted_by_running_it_again() {
    let root = scratch("door-again");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let cm = root.join("cm.zip");
    let day = stored_zip(&[("SYNTHA.csv", b"invented, never a vendor row\n")]);
    fs::write(&cm, cm_days(CmKind::Stocks, &["2024-04-01"], &day)).unwrap();
    let env = env_of(&[(CM_ARCHIVE_VAR, cm)]);
    let args = ["stocks", "zip", "2024-04-01", "2024-04-01", "NOSUCHSYMBOL"];
    let (code, out) = run(&args, &env, &store);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("GDFL IMPORT FAILED: no_file_read: the run owed 1 days"),
        "{out}"
    );
    for _ in 0..2 {
        let (code, out) = run(&args, &env, &store);
        assert!(out.contains("days already done   1"), "{out}");
        assert!(out.contains("files read          0"), "{out}");
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.contains(
                "GDFL IMPORT FAILED: no_file_read: the run read no instrument file and skipped 1 \
                 days as already done, and the last run of this key"
            ),
            "{out}"
        );
        let ledger = fs::read_to_string(sources_path(&store)).unwrap();
        let line = Entry::parse(ledger.lines().last().unwrap()).unwrap();
        assert_eq!(line.outcome, NO_FILE_READ, "{ledger}");
    }
    fs::remove_dir_all(root).unwrap();
}

/// The rule of [`nothing_read_again`] on its own: only a run that read no
/// file, skipped a done day, and whose key's last line ended `no_file_read`
/// (alone or beside another code; a word that merely begins alike is not
/// the code) is named.
#[test]
fn only_a_rerun_of_a_run_that_read_nothing_is_named() {
    let tail = |outcome: Option<&str>| Tail {
        last: outcome.map(|o| {
            let mut line = entry(&hex(&[1; 32]), 7);
            line.outcome = o.to_owned();
            line
        }),
        ..Tail::default()
    };
    let skipped = |files: usize, days_skipped: usize| Report {
        files,
        days_skipped,
        ..Report::default()
    };
    let named = |report: &Report, outcome: Option<&str>| {
        nothing_read_again(report, &tail(outcome)).map(|f| f.code)
    };
    assert_eq!(
        named(&skipped(0, 1), Some(NO_FILE_READ)),
        Some(NO_FILE_READ)
    );
    let both = format!("{NO_FILE_READ}+{SOURCE_LOST}");
    assert_eq!(named(&skipped(0, 3), Some(&both)), Some(NO_FILE_READ));
    let widened = Report {
        days_widened: 1,
        ..Report::default()
    };
    assert_eq!(named(&widened, Some(NO_FILE_READ)), Some(NO_FILE_READ));
    assert_eq!(named(&skipped(0, 1), Some("ok")), None);
    assert_eq!(named(&skipped(0, 1), Some("no_file_read_extra")), None);
    assert_eq!(named(&skipped(0, 1), None), None, "a first run");
    assert_eq!(
        named(&skipped(1, 1), Some(NO_FILE_READ)),
        None,
        "it read a file"
    );
    assert_eq!(
        named(&skipped(0, 0), Some(NO_FILE_READ)),
        None,
        "it skipped nothing"
    );
}

// ═══ 3. The end of a run ════════════════════════════════════════════════════

/// **AN EMPTY RANGE OWES NOTHING AND EXITS OK.** A weekend reads no file and
/// owes no day: that is not `no_file_read`.
#[test]
fn an_empty_range_owes_nothing_and_exits_ok() {
    let root = scratch("door-weekend");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let (code, out) = run(
        &["indices", "tickstore", "2024-04-06", "2024-04-07"],
        &env_of(&[(TICKSTORE_VAR, ts)]),
        &store,
    );
    assert_eq!(code, OK, "{out}");
    assert!(!out.contains(NO_FILE_READ), "{out}");
    let ledger = fs::read_to_string(sources_path(&store)).unwrap();
    assert_eq!(ledger.lines().count(), 1, "{ledger}");
    assert!(ledger.contains("\tok\t0\t0\t*\t"), "{ledger}");
    fs::remove_dir_all(root).unwrap();
}

/// **A VANISHED STORE ROOT IS NAMED `store_lost`.** The operator's volume
/// gone: `/Volumes` is root's, so nothing can recreate the mount and every
/// write fails "Permission denied". The door names the loss instead; and a
/// run that ended on journal failures with its store gone says so too.
#[test]
fn a_vanished_store_root_is_named_store_lost() {
    let root = scratch("door-storelost");
    let ts = root.join("ts");
    tick_store(&ts);
    let volumes = root.join("Volumes");
    fs::create_dir_all(&volumes).unwrap();
    let store = volumes.join("WD_BLACK").join("store");
    set_mode(&volumes, 0o555);
    let bound = modes_bind(&volumes);
    let (code, out) = run(
        &["indices", "tickstore", "2024-04-01", "2024-04-01"],
        &env_of(&[(TICKSTORE_VAR, ts.clone())]),
        &store,
    );
    set_mode(&volumes, 0o755);
    assert_eq!(code, FAILED, "{out}");
    if bound {
        assert!(
            out.contains("GDFL IMPORT REFUSED: store_lost: the store root ")
                && !out.contains("Permission denied"),
            "{out}"
        );
    }
    let mut report = nothing_read(&["2024-04-01"]);
    report.failures.push(pull::ingest::Failure::new(
        "2024-04-01",
        "journal: Permission denied (os error 13)",
    ));
    let found = verdict(Some(&report), &Identity::First, (&store, &ts));
    let codes: Vec<&str> = found.iter().map(|f| f.code).collect();
    assert_eq!(codes, [NO_FILE_READ, STORE_LOST]);
    fs::remove_dir_all(root).unwrap();
}

/// **A VANISHED SOURCE ROOT IS NAMED `source_lost`.** The root passed the
/// door, then went while the run read it (a second volume dropped): the end
/// of the run probes it once and names it. A root the host refuses to look
/// at is not called lost (that would be FM-086's `volume_access_revoked`).
#[test]
fn a_vanished_source_root_is_named_source_lost() {
    let root = scratch("door-sourcelost");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let origin = Origin::measure(SourceKind::TickStore, &ts, None);
    let door = Door::open(Opening {
        kind: ImportKind::Indices,
        var: TICKSTORE_VAR,
        store: &store,
        origin: &origin,
        only: &[],
        args: &["indices", "tickstore", "2024-04-01", "2024-04-01"],
    })
    .unwrap();
    fs::remove_dir_all(&ts).unwrap();
    let mut out = String::new();
    assert!(door.close(Some(&nothing_read(&["2024-04-01"])), None, &mut out));
    assert!(
        out.contains("GDFL IMPORT FAILED: no_file_read: ")
            && out.contains("GDFL IMPORT FAILED: source_lost: the source root "),
        "{out}"
    );
    let ledger = fs::read_to_string(sources_path(&store)).unwrap();
    assert!(ledger.contains("\tno_file_read+source_lost\t"), "{ledger}");

    let shut = root.join("shut");
    fs::create_dir_all(shut.join("inner")).unwrap();
    set_mode(&shut, 0o000);
    let bound = fs::metadata(shut.join("inner")).is_err();
    let lost = probe_roots(&store, &shut.join("inner"));
    set_mode(&shut, 0o755);
    if bound {
        assert!(lost.is_empty(), "a refused look is not a loss: {lost:?}");
    }
    fs::remove_dir_all(root).unwrap();
}

/// **A SOURCE ROOT THAT VANISHES AFTER FILES WERE READ IS NAMED
/// `source_lost` (verify-AF-01-r1 blocking 1).** The worst case of ingest-14:
/// the source's volume goes after 11 files were read, the remaining days
/// close "not in source", no failure is recorded and nothing owed went
/// unread, so the end-of-run probe, which ran only after a failure, never
/// ran: exit 0, outcome `ok`. The roots are probed at the end of every run
/// now. The same report with the root present is clean, so it is the probe
/// that fails the run, not the report.
#[test]
fn a_source_root_that_vanishes_after_files_were_read_is_named_source_lost() {
    let root = scratch("door-midrun");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let origin = Origin::measure(SourceKind::TickStore, &ts, None);
    let open = || {
        Door::open(Opening {
            kind: ImportKind::Indices,
            var: TICKSTORE_VAR,
            store: &store,
            origin: &origin,
            only: &[],
            args: &["indices", "tickstore", "2019-01-01", "2019-01-04"],
        })
        .unwrap()
    };
    let mut report = nothing_read(&["2019-01-03", "2019-01-04"]);
    report.days_imported = 2;
    report.files = 11;
    let mut out = String::new();
    assert!(!open().close(Some(&report), None, &mut out), "{out}");
    let renamed = root.join("ts-moved-aside");
    let door = open();
    fs::rename(&ts, &renamed).unwrap();
    let mut out = String::new();
    assert!(door.close(Some(&report), None, &mut out), "{out}");
    assert!(
        out.contains("GDFL IMPORT FAILED: source_lost: the source root ")
            && !out.contains("no_file_read"),
        "{out}"
    );
    let ledger = fs::read_to_string(sources_path(&store)).unwrap();
    let last = ledger.lines().last().unwrap();
    assert_eq!(Entry::parse(last).unwrap().outcome, SOURCE_LOST, "{ledger}");
    // The verb: the same run with the root back ends as its report says.
    fs::rename(&renamed, &ts).unwrap();
    let (code, out) = run(
        &["indices", "tickstore", "2024-04-06", "2024-04-07"],
        &env_of(&[(TICKSTORE_VAR, ts.clone())]),
        &store,
    );
    assert_eq!(code, OK, "{out}");
    assert!(!out.contains("source_lost"), "{out}");
    fs::remove_dir_all(root).unwrap();
}

/// **EVERY DAY ALREADY REFUSED IDENTICALLY STILL FAILS WHEN THE ROOT IS GONE
/// (ingest-14 correction 4).** A day journalled `refused day_not_held` adds
/// no failure when it is refused the same way again, so a run whose every
/// day was already refused used to exit 0. Twice over a root that holds
/// none of the days: `FAILED` both times. With the root gone at the door:
/// refused. With the root gone during the run: `source_lost` beside
/// `no_file_read`.
#[test]
fn every_day_already_refused_identically_still_fails_when_the_root_is_gone() {
    let root = scratch("door-identical");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let env = env_of(&[(TICKSTORE_VAR, ts.clone())]);
    let args = ["indices", "tickstore", "2024-04-01", "2024-04-02"];
    for _ in 0..2 {
        let (code, out) = run(&args, &env, &store);
        assert_eq!(code, FAILED, "{out}");
        assert!(out.contains("GDFL IMPORT FAILED: no_file_read: "), "{out}");
        assert!(out.contains("failures            0"), "{out}");
    }
    let refused = fs::read_to_string(journal_path(&store)).unwrap();
    assert_eq!(
        refused
            .lines()
            .filter(|l| l.starts_with("refused "))
            .count(),
        2,
        "the second run wrote no identical refusal: {refused}"
    );
    let origin = Origin::measure(SourceKind::TickStore, &ts, None);
    let door = Door::open(Opening {
        kind: ImportKind::Indices,
        var: TICKSTORE_VAR,
        store: &store,
        origin: &origin,
        only: &[],
        args: &args,
    })
    .unwrap();
    fs::remove_dir_all(&ts).unwrap();
    let mut out = String::new();
    assert!(door.close(
        Some(&nothing_read(&["2024-04-01", "2024-04-02"])),
        None,
        &mut out
    ));
    assert!(out.contains("source_lost: "), "{out}");
    let (code, out) = run(&args, &env, &store);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("source_root_unknown: ") && out.contains("does not exist"),
        "{out}"
    );
    fs::remove_dir_all(root).unwrap();
}

// ═══ 4. A source changed since the last run of its key ═════════════════════

/// **A CHANGED ARCHIVE WITH EVERY DAY DONE IS NAMED, NOT OK (FM-290's door
/// half, ingest-1).** Fixture A, every day of the range already done: OK,
/// its digest recorded. Fixture B at the same path with one member changed,
/// same key: every day is still skipped unread, and the run is `FAILED`
/// `source_changed_since_last_run` with both digests and the count. The
/// same B again is still `FAILED` against A (a line that named a change is
/// never the baseline); A restored at the path is OK again.
#[test]
fn a_changed_archive_with_every_day_done_is_named_not_ok() {
    let root = scratch("door-changed");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let definition = pull::gdfl_import::BAR_DEFINITION;
    fs::write(
        journal_path(&store),
        format!(
            "run 1700000000 source=zip\n\
             done stocks 2024-04-01 * definition={definition} run=1700000000\n\
             done stocks 2024-04-02 * definition={definition} run=1700000000\n"
        ),
    )
    .unwrap();
    let cm = root.join("cm.zip");
    let env = env_of(&[(CM_ARCHIVE_VAR, cm.clone())]);
    let args = ["stocks", "zip", "2024-04-01", "2024-04-02"];
    fs::write(&cm, cm_days(CmKind::Stocks, &TWO, &empty_zip())).unwrap();
    let (code, out) = run(&args, &env, &store);
    assert_eq!(code, OK, "{out}");
    // Both done days were not read again: skipped as done, or (WP-12f) only
    // their listing read for a wider scope. The door counts either unread.
    let ledger = fs::read_to_string(sources_path(&store)).unwrap();
    let first = Entry::parse(ledger.lines().last().unwrap()).unwrap();
    assert_eq!(first.days_skipped, 2, "{out}");
    assert!(out.contains("(no earlier run of this key in "), "{out}");
    let a = Origin::measure(SourceKind::Zip, &cm, None)
        .digest
        .map(|d| hex(&d))
        .unwrap();

    let mut changed = empty_zip();
    changed[20] = 1;
    changed.push(b'B');
    fs::write(&cm, cm_days(CmKind::Stocks, &TWO, &changed)).unwrap();
    let b = Origin::measure(SourceKind::Zip, &cm, None)
        .digest
        .map(|d| hex(&d))
        .unwrap();
    assert_ne!(a, b);
    let (code, out) = run(&args, &env, &store);
    assert_eq!(code, FAILED, "{out}");
    let want = format!(
        "GDFL IMPORT FAILED: source_changed_since_last_run: the archive's digest was {a} at the run of "
    );
    assert!(out.contains(&want), "{out}");
    assert!(
        out.contains(&format!("is {b} now; 2 days were skipped unread"))
            && out.contains("IA-6")
            && out.contains("R3-22"),
        "{out}"
    );
    // The same B again: still FAILED against A. Running again never accepts
    // a change whose skipped days were not landed.
    let (code, out) = run(&args, &env, &store);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains(&want), "{out}");
    assert!(
        out.contains("1 later runs of this key named this change already")
            && out.contains(&format!("(CHANGED: {a} at the run of ")),
        "{out}"
    );
    // A restored at the same path: the same as the baseline, OK.
    fs::write(&cm, cm_days(CmKind::Stocks, &TWO, &empty_zip())).unwrap();
    let (code, out) = run(&args, &env, &store);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(&format!("source identity     {a} (the same as the run of ")),
        "{out}"
    );
    let ledger = fs::read_to_string(sources_path(&store)).unwrap();
    assert_eq!(ledger.lines().count(), 4, "{ledger}");
    assert_eq!(
        ledger
            .lines()
            .filter(|l| l.contains("\tsource_changed_since_last_run\t"))
            .count(),
        2,
        "{ledger}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **A CHANGE IS NEVER ACCEPTED BY A RUN THAT SKIPPED NO DAY
/// (verify-AF-01-r1 blocking 2).** On a changed archive a run over a range
/// with no done day (a weekend, or the daily timer's new days) skipped
/// nothing unread and is not failed for it, but its line used to be written
/// `ok` with the new digest and become the baseline: the done days' next run
/// then said "the same" and exited 0. Its line now carries
/// `source_changed_unchecked`, the receipt says the change is unresolved,
/// and the done days' run still fails against the original digest, however
/// many such runs come between. The original archive restored is OK.
#[test]
fn a_change_is_never_accepted_by_a_run_that_skipped_no_day() {
    let root = scratch("door-unchecked");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let definition = pull::gdfl_import::BAR_DEFINITION;
    fs::write(
        journal_path(&store),
        format!("run 1700000000 source=zip\ndone stocks 2024-04-01 * definition={definition} run=1700000000\n"),
    )
    .unwrap();
    let cm = root.join("cm.zip");
    let env = env_of(&[(CM_ARCHIVE_VAR, cm.clone())]);
    let done = ["stocks", "zip", "2024-04-01", "2024-04-01"];
    let weekend = ["stocks", "zip", "2024-04-06", "2024-04-07"];
    fs::write(&cm, cm_days(CmKind::Stocks, &["2024-04-01"], &empty_zip())).unwrap();
    let (code, out) = run(&done, &env, &store);
    assert_eq!(code, OK, "{out}");
    let a = Origin::measure(SourceKind::Zip, &cm, None)
        .digest
        .map(|d| hex(&d))
        .unwrap();
    let mut changed = empty_zip();
    changed.push(b'B');
    fs::write(&cm, cm_days(CmKind::Stocks, &["2024-04-01"], &changed)).unwrap();
    for round in 1..=2 {
        let (code, out) = run(&weekend, &env, &store);
        assert_eq!(
            code, OK,
            "round {round}: a run that skipped nothing hid nothing: {out}"
        );
        assert!(
            out.contains(&format!("(CHANGED: {a} at the run of "))
                && out.contains("source change       unresolved and not checked by this run")
                && out.contains("never the baseline"),
            "round {round}: {out}"
        );
        let ledger = fs::read_to_string(sources_path(&store)).unwrap();
        let last = Entry::parse(ledger.lines().last().unwrap()).unwrap();
        assert_eq!(last.outcome, SOURCE_CHANGED_UNCHECKED, "{ledger}");
        assert_eq!(last.days_skipped, 0, "{ledger}");
    }
    // The done day: still FAILED against A, with both unchecked runs counted.
    for _ in 0..2 {
        let (code, out) = run(&done, &env, &store);
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.contains(&format!(
                "GDFL IMPORT FAILED: source_changed_since_last_run: the archive's digest was {a} at the run of "
            )),
            "{out}"
        );
        assert!(!out.contains("(the same as the run of"), "{out}");
    }
    // A restored: the same as the baseline, OK.
    fs::write(&cm, cm_days(CmKind::Stocks, &["2024-04-01"], &empty_zip())).unwrap();
    let (code, out) = run(&done, &env, &store);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(&format!("source identity     {a} (the same as the run of ")),
        "{out}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **ONE ARCHIVE REACHED BY ANOTHER SPELLING IS THE SAME KEY
/// (verify-AF-01-r1 X7).** The key's path was the variable's raw string, so
/// the same archive through a symbolic link, or with a doubled slash, was "no
/// earlier run of this key" and its changed digest was never compared: the
/// done day was skipped unread and the run exited 0. The key's path is now
/// the root as the host resolves it, so every spelling compares with the one
/// baseline and the change is named.
#[test]
fn one_archive_reached_by_another_spelling_is_the_same_key() {
    let root = scratch("door-spelling");
    let store = root.join("store");
    fs::create_dir_all(store.join("imports")).unwrap();
    let definition = pull::gdfl_import::BAR_DEFINITION;
    fs::write(
        journal_path(&store),
        format!("run 1700000000 source=zip\ndone stocks 2024-04-01 * definition={definition} run=1700000000\n"),
    )
    .unwrap();
    let cm = root.join("cm.zip");
    let done = ["stocks", "zip", "2024-04-01", "2024-04-01"];
    fs::write(&cm, cm_days(CmKind::Stocks, &["2024-04-01"], &empty_zip())).unwrap();
    let (code, out) = run(&done, &env_of(&[(CM_ARCHIVE_VAR, cm.clone())]), &store);
    assert_eq!(code, OK, "{out}");
    let canonical = fs::canonicalize(&cm).unwrap();
    let ledger = fs::read_to_string(sources_path(&store)).unwrap();
    let first = Entry::parse(ledger.lines().last().unwrap()).unwrap();
    assert_eq!(first.path, escape(canonical.as_os_str().as_bytes()));
    let a = first.digest;

    let mut changed = empty_zip();
    changed.push(b'B');
    fs::write(&cm, cm_days(CmKind::Stocks, &["2024-04-01"], &changed)).unwrap();
    let link = root.join("link-to-cm.zip");
    std::os::unix::fs::symlink(&cm, &link).unwrap();
    let doubled = PathBuf::from(format!("{}//cm.zip", root.display()));
    let dotted = root.join("store").join("..").join("cm.zip");
    for spelling in [link, doubled, dotted] {
        let (code, out) = run(
            &done,
            &env_of(&[(CM_ARCHIVE_VAR, spelling.clone())]),
            &store,
        );
        assert_eq!(code, FAILED, "{}: {out}", spelling.display());
        assert!(
            out.contains(&format!(
                "GDFL IMPORT FAILED: source_changed_since_last_run: the archive's digest was {a} at the run of "
            )),
            "{}: {out}",
            spelling.display()
        );
        assert!(!out.contains("no earlier run of this key"), "{out}");
    }
    let ledger = fs::read_to_string(sources_path(&store)).unwrap();
    assert!(
        ledger
            .lines()
            .filter_map(Entry::parse)
            .all(|e| e.path == escape(canonical.as_os_str().as_bytes())),
        "{ledger}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **A ROOT THAT CANNOT BE RESOLVED TO ONE PATH IS NAMED, NEVER KEYED BY ITS
/// RAW SPELLING.** A root that went between its check and the lock is
/// `source_lost`; one that is present but cannot be resolved is
/// `source_root_unknown`, naming the variable.
#[test]
fn a_root_that_cannot_be_resolved_is_named() {
    let root = scratch("door-canonical");
    let gone = root.join("never-made.zip");
    let lost = canonical_source(CM_ARCHIVE_VAR, &gone).unwrap_err();
    assert_eq!(lost.code, SOURCE_LOST, "{lost}");
    assert!(
        lost.words
            .contains("went between its check and the run lock")
    );
    let cm = root.join("cm.zip");
    fs::write(&cm, b"x").unwrap();
    assert_eq!(
        canonical_source(CM_ARCHIVE_VAR, &cm).unwrap(),
        fs::canonicalize(&cm).unwrap()
    );
    // A loop of links exists by name but resolves to nothing.
    let a = root.join("loop-a");
    let b = root.join("loop-b");
    std::os::unix::fs::symlink(&b, &a).unwrap();
    std::os::unix::fs::symlink(&a, &b).unwrap();
    let looped = canonical_source(CM_ARCHIVE_VAR, &a).unwrap_err();
    assert_eq!(looped.code, SOURCE_ROOT_UNKNOWN, "{looped}");
    assert!(
        looped.words.starts_with(&format!(
            "{CM_ARCHIVE_VAR}={} cannot be resolved",
            shown(&a)
        )),
        "{looped}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **A DONE DAY READ AGAIN ONLY FOR A WIDER SCOPE COUNTS AS SKIPPED UNREAD.**
/// Since WP-12f (D-4878) a done day whose close did not cover the run's
/// scope has its listing read again for the members it never covered, and
/// is counted in `days_widened`, not `days_skipped`: its done members are
/// still not re-read. Counting only `days_skipped` let a changed archive
/// with every day widened through as unchecked; the door counts both.
#[test]
fn a_widened_done_day_counts_as_skipped_unread() {
    let changed = Identity::Changed {
        old: hex(&[1; 32]),
        new: hex(&[2; 32]),
        since: 1,
        unresolved: 0,
    };
    let roots = (Path::new("/"), Path::new("/"));
    let mut report = nothing_read(&[]);
    report.days_widened = 2;
    assert_eq!(unread(&report), 2);
    let found = verdict(Some(&report), &changed, roots);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, SOURCE_CHANGED);
    assert!(
        found[0].words.contains("2 days were skipped unread"),
        "{}",
        found[0]
    );
    report.days_skipped = 1;
    assert_eq!(unread(&report), 3);
    report.days_widened = 0;
    report.days_skipped = 0;
    assert!(verdict(Some(&report), &changed, roots).is_empty());
}

/// **A TICK-STORE SOURCE SAYS ITS IDENTITY IS NOT COMPARABLE.** Its digest is
/// `FORMAT.md`'s, so a re-delivered day could not show; the receipt says so
/// rather than claim "the same".
#[test]
fn a_tickstore_source_says_its_identity_is_not_comparable() {
    let root = scratch("door-ts-identity");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    for _ in 0..2 {
        let (_, out) = run(
            &["indices", "tickstore", "2024-04-06", "2024-04-06"],
            &env_of(&[(TICKSTORE_VAR, ts.clone())]),
            &store,
        );
        assert!(
            out.contains(&format!("source identity     {NOT_COMPARABLE}")),
            "{out}"
        );
        assert!(!out.contains(SOURCE_CHANGED), "{out}");
    }
    fs::remove_dir_all(root).unwrap();
}

// ═══ 5. The sources ledger ══════════════════════════════════════════════════

fn entry(digest: &str, started: u64) -> Entry {
    Entry {
        started,
        kind: "stocks".to_owned(),
        source: "zip".to_owned(),
        digest: digest.to_owned(),
        outcome: "ok".to_owned(),
        days_skipped: 2,
        files: 0,
        key: "*".to_owned(),
        path: escape(b"/v/NSE (Stock+Indices) x.zip"),
    }
}

const KEY: Key<'static> = Key {
    kind: "stocks",
    source: "zip",
    filter: "*",
    path: "/v/NSE (Stock+Indices) x.zip",
};

/// A line round-trips; a path or key with a tab, a newline or a byte that is
/// not text stays one field.
#[test]
fn a_ledger_line_round_trips_and_escapes_its_fields() {
    let digest = "ab".repeat(32);
    let line = entry(&digest, 7).line();
    assert_eq!(
        Entry::parse(line.trim_end_matches('\n')),
        Some(entry(&digest, 7))
    );
    assert_eq!(escape(b"a\tb\nc\\d\xff"), "a\\tb\\nc\\\\d\\xff");
    for bad in [
        "v2\t7\tstocks\tzip\tunmeasured\tok\t2\t0\t*\t/p",
        "v1\t7\tstocks\tzip\tnot-a-digest\tok\t2\t0\t*\t/p",
        "v1\tx\tstocks\tzip\tunmeasured\tok\t2\t0\t*\t/p",
        "v1\t7\tstocks\tzip\tunmeasured\tok\t2\t0\t*\t/p\textra",
        "v1\t7\tstocks",
    ] {
        assert_eq!(Entry::parse(bad), None, "{bad}");
    }
    assert_eq!(filter_key(&[]), "*");
    assert_eq!(
        filter_key(&["TCS".to_owned(), "ACC".to_owned(), "TCS".to_owned()]),
        "ACC,TCS"
    );
    let long: Vec<String> = (0..2000).map(|_| "W".to_owned()).collect();
    let verb: Vec<&str> = long.iter().map(String::as_str).collect();
    let cut = verb_of(&verb);
    assert!(cut.len() < VERB_CAP + 32 && cut.contains("...(+"), "{cut}");
}

/// **A TORN LAST LINE IS NAMED AND SKIPPED, NEVER A PANIC; DUPLICATES
/// COMPARE AGAINST THE NEWEST; AN ABSENT LEDGER IS A FIRST RUN.**
#[test]
fn the_ledger_tail_survives_torn_duplicate_and_absent_lines() {
    let root = scratch("door-tail");
    let ledger = root.join(SOURCES);
    let absent = tail_for(&ledger, KEY).unwrap();
    assert!(absent.absent && absent.last.is_none());
    assert_eq!(
        identity(SourceKind::Zip, Some(&[1; 32]), &absent),
        Identity::First
    );

    let old = hex(&[1; 32]);
    let new = hex(&[2; 32]);
    let mut text = entry(&old, 1).line() + &entry(&new, 2).line() + &entry(&new, 2).line();
    text.push_str("garbage line\n");
    text.push_str(&entry(UNMEASURED, 3).line());
    text.push_str("v1\t9\tstocks\tzip\t");
    fs::write(&ledger, &text).unwrap();
    let tail = tail_for(&ledger, KEY).unwrap();
    assert_eq!(tail.torn, "v1\t9\tstocks\tzip\t".len());
    assert_eq!(tail.unreadable, 1);
    assert_eq!(tail.last, Some(entry(&new, 2)), "the newest measured line");
    assert_eq!(
        identity(SourceKind::Zip, Some(&[2; 32]), &tail),
        Identity::Same { since: 2 }
    );
    assert_eq!(
        identity(SourceKind::Zip, Some(&[3; 32]), &tail),
        Identity::Changed {
            old: new.clone(),
            new: hex(&[3; 32]),
            since: 2,
            unresolved: 0
        }
    );
    assert_eq!(identity(SourceKind::Zip, None, &tail), Identity::Unmeasured);
    assert_eq!(
        identity(SourceKind::TickStore, Some(&[2; 32]), &tail),
        Identity::NotComparable
    );
    // The append after a torn line starts a line of its own.
    append(&ledger, &entry(&old, 4).line()).unwrap();
    let after = tail_for(&ledger, KEY).unwrap();
    assert_eq!(after.torn, 0);
    assert_eq!(after.last, Some(entry(&old, 4)));
    let text = fs::read_to_string(&ledger).unwrap();
    assert!(
        text.ends_with(&format!("v1\t9\tstocks\tzip\t\n{}", entry(&old, 4).line())),
        "the torn bytes are a line of their own: {text}"
    );
    // Seen from a key the newest line does not match, the old torn bytes now
    // count as one unreadable line beside the garbage one.
    let other = Key {
        kind: "indices",
        ..KEY
    };
    assert_eq!(tail_for(&ledger, other).unwrap().unreadable, 2);

    // A link at the ledger's name is refused, never followed.
    let link = root.join("linked");
    std::os::unix::fs::symlink(&ledger, &link).unwrap();
    assert!(tail_for(&link, KEY).unwrap_err().contains("symbolic link"));
    fs::remove_dir_all(root).unwrap();
}

/// **THE TAIL READ IS BOUNDED AND SAYS WHEN IT WAS CUT.** A ledger past
/// [`TAIL_WINDOW`] is read from its last window only; a key whose only line
/// is older is "no earlier run in the window", never a whole-file scan.
#[test]
fn the_tail_window_is_bounded_and_says_when_it_was_cut() {
    let root = scratch("door-window");
    let ledger = root.join(SOURCES);
    let mut text = entry(&hex(&[1; 32]), 1).line();
    let mut other = entry(&hex(&[5; 32]), 2);
    other.kind = "options".to_owned();
    let filler = other.line();
    while (text.len() as u64) < TAIL_WINDOW + 4096 {
        text.push_str(&filler);
    }
    fs::write(&ledger, &text).unwrap();
    let tail = tail_for(&ledger, KEY).unwrap();
    assert!(tail.cut && tail.last.is_none() && tail.torn == 0 && tail.unreadable == 0);
    let window = &text.as_bytes()[text.len() - usize::try_from(TAIL_WINDOW).unwrap()..];
    assert_eq!(scan(window, true, KEY), tail);
    fs::remove_dir_all(root).unwrap();
}

/// **A LINE THAT NAMED A CHANGE IS NEVER THE BASELINE.** Lines whose outcome
/// names `source_changed_since_last_run`, alone or beside the run's own end,
/// are passed over and counted; the baseline is the line before them. When
/// every line of the key in the window named one, the change is still
/// unresolved and its run fails, never reads as a first run.
#[test]
fn a_line_that_named_a_change_is_never_the_baseline() {
    let a = hex(&[1; 32]);
    let b = hex(&[2; 32]);
    let named = |digest: &str, started: u64, outcome: &str| {
        let mut line = entry(digest, started);
        line.outcome = outcome.to_owned();
        line.line()
    };
    let text = entry(&a, 1).line()
        + &named(&b, 2, SOURCE_CHANGED)
        + &named(&b, 3, &format!("{SOURCE_CHANGED}+failed"))
        + &named(&b, 4, "no_file_read_extra");
    // The newest line is no SOURCE_CHANGED (a word that merely begins alike
    // is not the code): it is the baseline.
    let tail = scan(text.as_bytes(), false, KEY);
    assert_eq!(tail.unresolved, 0);
    assert_eq!(tail.last.as_ref().map(|e| e.started), Some(4));
    let text = entry(&a, 1).line()
        + &named(&b, 2, SOURCE_CHANGED)
        + &named(&b, 3, &format!("{SOURCE_CHANGED}+failed"));
    let tail = scan(text.as_bytes(), false, KEY);
    assert_eq!(tail.unresolved, 2);
    assert_eq!(tail.last, Some(entry(&a, 1)));
    assert_eq!(
        identity(SourceKind::Zip, Some(&[2; 32]), &tail),
        Identity::Changed {
            old: a.clone(),
            new: b.clone(),
            since: 1,
            unresolved: 2
        }
    );
    // A restored: the same as the baseline.
    assert_eq!(
        identity(SourceKind::Zip, Some(&[1; 32]), &tail),
        Identity::Same { since: 1 }
    );
    // The baseline beyond the window: unresolved, and a run that skipped a
    // day fails; one that skipped none owes nothing to compare.
    let only = named(&b, 2, SOURCE_CHANGED);
    let tail = scan(only.as_bytes(), true, KEY);
    assert_eq!(
        tail.unresolved, 0,
        "the first line of a cut window is not whole"
    );
    let tail = scan(format!("x\n{only}").as_bytes(), true, KEY);
    assert_eq!(tail.unresolved, 1);
    let unresolved = identity(SourceKind::Zip, Some(&[2; 32]), &tail);
    assert_eq!(unresolved, Identity::Unresolved { unresolved: 1 });
    let roots = (Path::new("/"), Path::new("/"));
    let mut report = nothing_read(&[]);
    report.files = 1;
    report.days_skipped = 3;
    let found = verdict(Some(&report), &unresolved, roots);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, SOURCE_CHANGED);
    assert!(
        found[0].words.contains("1 runs of this key"),
        "{}",
        found[0]
    );
    assert!(!unchecked(&unresolved, &found), "the change was checked");
    report.days_skipped = 0;
    let found = verdict(Some(&report), &unresolved, roots);
    assert!(found.is_empty(), "{found:?}");
    // A run that skipped no day hid none, but its line carries the unchecked
    // word, so it is passed over exactly as a line that named the change.
    assert!(unchecked(&unresolved, &found));
    assert_eq!(
        outcome(&found, true, None),
        SOURCE_CHANGED_UNCHECKED,
        "never `ok`"
    );
    assert_eq!(
        outcome(&found, true, Some("failed")),
        format!("{SOURCE_CHANGED_UNCHECKED}+failed")
    );
    assert!(!unchecked(&Identity::Same { since: 1 }, &found));
    assert!(!unchecked(&Identity::First, &found));
    let text = entry(&a, 1).line()
        + &named(&b, 2, SOURCE_CHANGED)
        + &named(&b, 3, SOURCE_CHANGED_UNCHECKED)
        + &named(&b, 4, &format!("no_file_read+{SOURCE_CHANGED_UNCHECKED}"));
    let tail = scan(text.as_bytes(), false, KEY);
    assert_eq!(
        tail.unresolved, 3,
        "an unchecked line is never the baseline"
    );
    assert_eq!(tail.last, Some(entry(&a, 1)));
    // A word that merely begins alike is not the word.
    let text = entry(&a, 1).line() + &named(&b, 2, "source_changed_unchecked_x");
    let tail = scan(text.as_bytes(), false, KEY);
    assert_eq!(tail.last.as_ref().map(|e| e.started), Some(2));
}

/// A writer that refuses as a full disk does.
struct Full;

impl io::Write for Full {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(ErrorKind::StorageFull))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// **A FULL DISK ON THE LEDGER APPEND IS FAILED AND NAMED.** The append is
/// one write after a newline when the last line was torn; a refused append is
/// `sources_ledger_unwritten`, a full disk said as one, and the door's end
/// is then `FAILED`.
#[test]
fn a_full_disk_on_the_ledger_append_is_failed_and_named() {
    let mut written = Vec::new();
    append_to(&mut written, true, "v1 line\n").unwrap();
    assert_eq!(written, b"\nv1 line\n");
    let refused = append_to(&mut Full, false, "v1 line\n").unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::StorageFull);
    let named = unwritten(Path::new("/s/imports/gdfl.sources"), &refused);
    assert_eq!(named.code, SOURCES_LEDGER);
    assert!(named.words.contains("the disk is full"), "{named}");

    let root = scratch("door-full");
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let ts = root.join("ts");
    tick_store(&ts);
    let origin = Origin::measure(SourceKind::TickStore, &ts, None);
    let door = Door::open(Opening {
        kind: ImportKind::Indices,
        var: TICKSTORE_VAR,
        store: &store,
        origin: &origin,
        only: &[],
        args: &[],
    })
    .unwrap();
    // The ledger's name taken by a directory: the append is refused.
    fs::create_dir_all(sources_path(&store)).unwrap();
    let mut out = String::new();
    assert!(door.close(Some(&Report::default()), None, &mut out));
    assert!(
        out.contains("GDFL IMPORT FAILED: sources_ledger_unwritten: "),
        "{out}"
    );
    assert_eq!(
        fs::read(store.join("imports/gdfl.lock")).unwrap(),
        b"",
        "released"
    );
    fs::remove_dir_all(root).unwrap();
}

/// The lost codes are the words the store and the alerts already write, so
/// a reader of either finds them.
#[test]
fn the_lost_codes_are_the_words_the_store_and_the_alerts_already_use() {
    assert_eq!(
        pull::alerts::KINDS
            .iter()
            .find(|(n, _)| *n == 2)
            .map(|k| k.1),
        Some(STORE_LOST)
    );
    assert_eq!(store::verdict::DayState::SourceLost.word(), SOURCE_LOST);
    assert_eq!(outcome(&[], false, None), "ok");
    assert_eq!(
        outcome(
            &[Finding::new(NO_FILE_READ, String::new())],
            false,
            Some("failed")
        ),
        "no_file_read+failed"
    );
    // The range rule moved into the one run authority (IA-1, D-4800): a
    // one-day range is no misuse.
    let same = Day::new(2024, 4, 1).unwrap();
    let one_day = pull::gdfl_run::Request::new(
        pull::gdfl_run::Kinds::one(ImportKind::Indices),
        pull::gdfl_run::SourceWord::TickStore,
        (same, same),
    );
    assert_eq!(one_day.misuse(), None);
}

// ═══ 6. What the door costs ═════════════════════════════════════════════════

/// p50, p99 and max of `n` timings, in microseconds.
fn spread(mut micros: Vec<u128>) -> (u128, u128, u128) {
    micros.sort_unstable();
    let n = micros.len();
    (
        micros[n / 2],
        micros[(n * 99 / 100).min(n - 1)],
        micros[n - 1],
    )
}

fn timed(n: usize, mut each: impl FnMut()) -> (u128, u128, u128) {
    let mut micros = Vec::with_capacity(n);
    for _ in 0..n {
        let at = Instant::now();
        each();
        micros.push(at.elapsed().as_micros());
    }
    spread(micros)
}

/// **WHAT THE DOOR COSTS PER RUN, MEASURED (AF-01's O(1) proof).** Run by
/// hand: `AF01_BENCH_STORE` is a store root this test may write (a shadow's,
/// never the real store), and `AF01_BENCH_TICKSTORE`, `AF01_BENCH_NFO` and
/// `AF01_BENCH_CM` are source roots it only reads. n = 200 of each: the lock
/// taken and released, each root probed, the tail of a ledger longer than
/// the window read, and one line appended and made durable.
#[test]
#[ignore = "a measurement run by hand against a shadow store and the source roots"]
fn door_costs_on_a_real_layout() {
    const N: usize = 200;
    let var = |name: &str| PathBuf::from(std::env::var_os(name).expect(name));
    let store = var("AF01_BENCH_STORE");
    assert!(
        !store.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
        "never the real store"
    );
    let mut rows: Vec<(String, (u128, u128, u128))> = Vec::new();
    let lock = timed(N, || {
        RunLock::take(&store, &holder(ImportKind::Options))
            .unwrap()
            .release()
            .unwrap();
    });
    rows.push(("lock take + record + release".to_owned(), lock));
    for (name, shape, root) in [
        ("tick store", Shape::TickStore, var("AF01_BENCH_TICKSTORE")),
        ("options zips", Shape::NfoYears, var("AF01_BENCH_NFO")),
        (
            "CM outer zip",
            Shape::CmArchive(CmKind::Stocks),
            var("AF01_BENCH_CM"),
        ),
    ] {
        rows.push((
            format!("root check, {name}"),
            timed(N, || assert_eq!(root_fault(shape, &root), None)),
        ));
        rows.push((
            format!("end probe, store + {name}"),
            timed(N, || assert!(probe_roots(&store, &root).is_empty())),
        ));
        rows.push((
            format!("key path resolved, {name}"),
            timed(N, || assert!(canonical_source("BENCH", &root).is_ok())),
        ));
    }
    let ledger = sources_path(&store);
    let mut other = entry(&hex(&[5; 32]), 2);
    other.kind = "options".to_owned();
    let filler = other.line();
    let mut text = String::new();
    while (text.len() as u64) < 2 * TAIL_WINDOW {
        text.push_str(&filler);
    }
    fs::write(&ledger, &text).unwrap();
    rows.push((
        "ledger tail read (2 MiB ledger)".to_owned(),
        timed(N, || assert!(tail_for(&ledger, KEY).unwrap().cut)),
    ));
    rows.push((
        "ledger append + sync_data".to_owned(),
        timed(N, || append(&ledger, &filler).unwrap()),
    ));
    for (name, (p50, p99, max)) in &rows {
        println!("AF01 {name:<34} n={N} p50={p50}us p99={p99}us max={max}us");
        assert!(*p99 < 1_000_000, "{name}: p99 {p99}us");
    }
}
