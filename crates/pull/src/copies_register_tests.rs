//! Tests of the copies register (CC-5, FM-607; D-7040..D-7048).
//!
//! Every fixture is invented: folder names, bytes, devices and inodes. Real
//! trees are built in a scratch folder this file makes and removes; the
//! cases a host cannot stage (two devices, a name that vanishes between a
//! listing and its `lstat`, a folder that refuses its listing, one folder
//! reached by two names) run on [`Fake`], an in-memory tree behind the same
//! [`Fs`] seam the register reads through.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail"
)]

use std::collections::{BTreeMap, HashMap};
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use super::*;

// ═══ Fixtures ══════════════════════════════════════════════════════════════

/// A new, empty scratch folder for one test.
fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "brutex-copies-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Writes `bytes` at `path`, making its folders.
fn put(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// A live root of `kind` at `path`.
fn live(kind: LiveKind, path: &Path) -> LiveRoot {
    LiveRoot {
        kind,
        path: path.to_path_buf(),
        named_by: format!("test {}", kind.word()),
    }
}

/// A configuration of `copies` and `lives`.
fn config_of(copies: &[&Path], lives: Vec<LiveRoot>) -> Config {
    Config {
        copy_roots: copies.iter().map(|path| path.to_path_buf()).collect(),
        live: lives,
        unset_live: Vec::new(),
    }
}

/// The register of `config` on the host, walked at a fixed invented time.
fn host_walk(config: &Config) -> Register {
    walk(&Host, config, 1_700_000_000_000_000)
}

/// Every file and folder under `root` with what a change would alter:
/// kind, size, inode, link count and modification time.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (bool, u64, u64, u64, i64, i64)> {
    use std::os::unix::fs::MetadataExt as _;
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                stack.push(path.clone());
            }
            out.insert(
                path,
                (
                    meta.is_dir(),
                    meta.size(),
                    meta.ino(),
                    meta.nlink(),
                    meta.mtime(),
                    meta.mtime_nsec(),
                ),
            );
        }
    }
    out
}

/// One node of [`Fake`].
#[derive(Debug, Clone)]
struct Node {
    meta: Meta,
    /// A folder's names, or the refusal of its listing.
    children: Option<Result<Vec<OsString>, String>>,
    /// A file's bytes.
    bytes: Vec<u8>,
}

/// An in-memory tree behind [`Fs`], counting every call.
#[derive(Debug, Default)]
struct Fake {
    nodes: Mutex<HashMap<PathBuf, Node>>,
    lstats: AtomicUsize,
    lists: AtomicUsize,
    reads: AtomicUsize,
    /// A path whose `lstat` waits while [`Fake::holding`] is set.
    hold: Mutex<Option<PathBuf>>,
    holding: AtomicBool,
    held: AtomicBool,
    /// Paths whose `lstat` the host refuses with something other than "not
    /// found".
    broken: Mutex<Vec<PathBuf>>,
}

/// A `Meta` of `kind`.
const fn meta(kind: Kind, dev: u64, ino: u64, nlink: u64, size: u64) -> Meta {
    Meta {
        kind,
        dev,
        ino,
        nlink,
        size,
        mtime_s: 1_700_000_000,
        mtime_ns: 7,
        ctime_s: 1_700_000_001,
        ctime_ns: 9,
    }
}

impl Fake {
    /// Adds `path` as `node`, and its name to its parent's listing.
    fn add(&self, path: &str, node: Node) {
        let path = PathBuf::from(path);
        let mut nodes = self.nodes.lock().unwrap();
        if let (Some(parent), Some(name)) = (path.parent(), path.file_name())
            && let Some(Node {
                children: Some(Ok(names)),
                ..
            }) = nodes.get_mut(parent)
        {
            names.push(name.to_owned());
        }
        nodes.insert(path, node);
    }

    fn dir(&self, path: &str, dev: u64, ino: u64) {
        self.add(
            path,
            Node {
                meta: meta(Kind::Dir, dev, ino, 2, 0),
                children: Some(Ok(Vec::new())),
                bytes: Vec::new(),
            },
        );
    }

    fn file(&self, path: &str, dev: u64, ino: u64, nlink: u64, bytes: &[u8]) {
        self.add(
            path,
            Node {
                meta: meta(
                    Kind::File,
                    dev,
                    ino,
                    nlink,
                    u64::try_from(bytes.len()).unwrap(),
                ),
                children: None,
                bytes: bytes.to_vec(),
            },
        );
    }

    /// A name listed in `parent` that `lstat` does not find: it vanished
    /// between the listing and the look.
    fn vanished(&self, parent: &str, name: &str) {
        let mut nodes = self.nodes.lock().unwrap();
        if let Some(Node {
            children: Some(Ok(names)),
            ..
        }) = nodes.get_mut(Path::new(parent))
        {
            names.push(OsString::from(name));
        }
    }

    /// The listing of `path` refuses.
    fn refuse_listing(&self, path: &str) {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.get_mut(Path::new(path)).unwrap().children =
            Some(Err("permission denied (invented)".to_owned()));
    }

    /// `name` in `parent` is a second name of the folder at `target`.
    fn second_name_of_dir(&self, parent: &str, name: &str, target: &str) {
        let node = self.nodes.lock().unwrap()[Path::new(target)].clone();
        self.add(&format!("{parent}/{name}"), node);
    }
}

impl Fs for Fake {
    fn lstat(&self, path: &Path) -> io::Result<Meta> {
        self.lstats.fetch_add(1, Ordering::SeqCst);
        if self
            .broken
            .lock()
            .unwrap()
            .iter()
            .any(|broken| broken == path)
        {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        if self.hold.lock().unwrap().as_deref() == Some(path) {
            self.held.store(true, Ordering::SeqCst);
            while self.holding.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        self.nodes
            .lock()
            .unwrap()
            .get(path)
            .map(|node| node.meta)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn list(&self, dir: &Path, each: &mut dyn FnMut(OsString)) -> io::Result<()> {
        self.lists.fetch_add(1, Ordering::SeqCst);
        let children = self
            .nodes
            .lock()
            .unwrap()
            .get(dir)
            .and_then(|node| node.children.clone());
        match children {
            Some(Ok(names)) => {
                for name in names {
                    each(name);
                }
                Ok(())
            }
            Some(Err(why)) => Err(io::Error::other(why)),
            None => Err(io::Error::from(io::ErrorKind::NotADirectory)),
        }
    }

    fn lines(&self, path: &Path, each: &mut dyn FnMut(&[u8])) -> Result<Ending, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        let bytes = self
            .nodes
            .lock()
            .unwrap()
            .get(path)
            .map(|node| node.bytes.clone())
            .ok_or_else(|| "absent".to_owned())?;
        if bytes.is_empty() {
            return Ok(Ending::Empty);
        }
        let torn = bytes.last() != Some(&b'\n');
        let body = if torn {
            &bytes[..]
        } else {
            &bytes[..bytes.len() - 1]
        };
        for line in body.split(|b| *b == b'\n') {
            each(line);
        }
        Ok(if torn {
            Ending::Torn
        } else {
            Ending::LineBreak
        })
    }
}

/// The words a command that deletes, moves or renames would use.
const COMMAND_WORDS: [&str; 14] = [
    "rm ", "rm\t", "rmdir", "unlink", "shred", "trash", "mv ", "rename", "del ", "delete",
    "truncate", "sudo", "find ", "xargs",
];

/// `text` with every quoted path blanked, so a folder whose own name holds
/// a command word is not read as a command.
fn unquoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut inside = false;
    let mut escaped = false;
    for c in text.chars() {
        if inside {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                inside = false;
            }
            out.push(' ');
        } else if c == '"' {
            inside = true;
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

// ═══ The eight tests the spec names ════════════════════════════════════════

/// A partial-download name that is a hard link of a live archive's year zip
/// is ONE red row: its path, the archive's name, `(st_dev, st_ino)`, the
/// link count, flagged partial; one `error` event that the audit stream
/// keeps; and a hazard row on the retirement list that frees no byte. The
/// unlinked partial beside it is the root's own bytes; the linked one's are
/// the archive's (FM-607; D-7042).
#[test]
fn a_hard_linked_part_name_to_a_configured_archive_raises_one_warning() {
    let base = scratch("hazard");
    let archive = base.join("Options");
    let queue = base.join("work/options-queue");
    let year = archive.join("2020.zip");
    put(&year, b"invented year zip bytes, not a vendor's");
    std::fs::create_dir_all(&queue).unwrap();
    let linked = queue.join("2020.preserved.17.zip.part");
    std::fs::hard_link(&year, &linked).unwrap();
    put(&queue.join("2021.zip.part"), b"an invented partial");
    let register = host_walk(&config_of(
        &[&queue],
        vec![live(LiveKind::NfoArchives, &archive)],
    ));
    assert_eq!(register.hazards.len(), 1, "{:?}", register.hazards);
    let hazard = register.hazards.first().unwrap();
    assert_eq!(hazard.path, linked);
    assert_eq!(hazard.live, year);
    assert_eq!(hazard.kind, LiveKind::NfoArchives);
    assert_eq!(hazard.nlink, 2);
    assert!(hazard.partial);
    {
        use std::os::unix::fs::MetadataExt as _;
        let meta = std::fs::metadata(&year).unwrap();
        assert_eq!((hazard.dev, hazard.ino), (meta.dev(), meta.ino()));
    }
    let root = &register.roots[0];
    assert_eq!(root.linked_live, 1);
    assert_eq!(root.extra_bytes, b"an invented partial".len() as u64);
    assert_eq!(
        root.bytes,
        (b"an invented partial".len() + b"invented year zip bytes, not a vendor's".len()) as u64
    );
    assert!(root.retirable(), "the folder itself is reader-less");
    let errors: Vec<Note> = notes(&register)
        .into_iter()
        .filter(|note| note.level == telemetry::Level::Error)
        .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(telemetry::is_audited(
        errors[0].level,
        TARGET,
        errors[0].message
    ));
    assert!(
        errors[0]
            .fields
            .iter()
            .any(|(k, v)| *k == "ino" && *v == hazard.ino.to_string())
    );
    let list = retirement(&register);
    let red: Vec<&Retire> = list.iter().filter(|r| r.hazard).collect();
    assert_eq!(red.len(), 1);
    assert_eq!(red[0].bytes, 0);
    assert!(
        red[0]
            .caution
            .as_deref()
            .unwrap()
            .contains("leaves the live file's data intact")
    );
    assert!(
        register.whole(),
        "the hazard is named, and the walk was whole"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// A copy root nothing names is listed with every file's bytes (nested
/// folders included), its file count and newest time, no reader, and a line
/// on the retirement list freeing exactly those bytes (FM-607 "done" 1).
#[test]
fn a_copy_with_no_reader_is_listed_with_its_bytes() {
    let base = scratch("no-reader");
    let copy = base.join("extracted");
    put(&copy.join("NSE_Tick/INDICES/a.csv"), &[b'x'; 300]);
    put(&copy.join("NSE_Tick/STOCKS/b.csv"), &[b'y'; 700]);
    put(&copy.join("c.csv"), &[b'z'; 11]);
    let store = base.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let register = host_walk(&config_of(&[&copy], vec![live(LiveKind::Store, &store)]));
    let root = &register.roots[0];
    assert_eq!(root.state, State::Walked { whole: true });
    assert_eq!(root.files, 3);
    assert_eq!(root.dirs, 3);
    assert_eq!(root.bytes, 1_011);
    assert_eq!(root.extra_bytes, 1_011);
    assert!(root.newest_mtime_s.is_some());
    assert_eq!(root.reader, None);
    assert_eq!(root.inside, None);
    assert_eq!(register.extra_bytes, 1_011);
    let list = retirement(&register);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].path, copy);
    assert_eq!(list[0].bytes, 1_011);
    assert_eq!(list[0].files, 3);
    assert!(
        list[0].twin.contains("may be the ONLY copy"),
        "{}",
        list[0].twin
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// A receipt in a copy root is named as its proof with what it states; a
/// `VERIFIED.tsv` whose every `source_identity` matches a live archive file
/// today proves a twin, and the retirement list says so (D-7045).
#[test]
fn a_receipt_in_a_copy_root_is_named_as_its_proof() {
    use std::os::unix::fs::MetadataExt as _;
    let base = scratch("receipt");
    let archive = base.join("Options");
    let year = archive.join("2024.zip");
    put(&year, b"invented year zip, 2024");
    let meta = std::fs::metadata(&year).unwrap();
    let identity = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        meta.dev(),
        meta.ino(),
        meta.size(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec()
    );
    let copy = base.join("options-nested-proof");
    let statement = "every configured archive passed nested ZIP structure and CRC checks";
    put(
        &copy.join("run-1/ALL_VERIFIED.txt"),
        format!("{statement}\n").as_bytes(),
    );
    put(
        &copy.join("2024/VERIFIED.tsv"),
        format!("day\tsource_identity\n2024-01-01\t{identity}\n2024-01-02\t{identity}\n")
            .as_bytes(),
    );
    put(&copy.join("2024/day-1.zip"), b"an invented day zip");
    let register = host_walk(&config_of(
        &[&copy],
        vec![live(LiveKind::NfoArchives, &archive)],
    ));
    let root = &register.roots[0];
    assert_eq!(root.receipt_count, 2);
    assert_eq!(
        root.proof.as_deref(),
        Some(copy.join("2024/VERIFIED.tsv").as_path())
    );
    let all = root
        .receipts
        .iter()
        .find(|r| r.kind == ReceiptKind::AllVerified)
        .unwrap();
    assert_eq!(all.verdict, "states");
    assert_eq!(all.statement, statement);
    let tsv = root
        .receipts
        .iter()
        .find(|r| r.kind == ReceiptKind::VerifiedTsv)
        .unwrap();
    assert_eq!((tsv.sources, tsv.matched), (2, 2));
    assert_eq!(root.twin, Twin::Proven { sources: 2 });
    assert!(retirement(&register)[0].twin.contains("a proven twin"));
    let _ = std::fs::remove_dir_all(&base);
}

/// A RECEIPT NAMING A HARD-LINKED LIVE ARCHIVE still proves its twin: the
/// operator's year zips each have a second name (nlink 2, a `.zip.part` in
/// the queue), so the live walk holds them among its shared files, and a
/// `source_identity` must find them there. A tick-store file with a second
/// name is never an archive: a receipt naming it proves nothing.
/// COMBINATION: a hazard, a proven twin and an unproven one in one walk.
#[test]
fn a_receipt_naming_a_hard_linked_archive_proves_its_twin_and_a_tick_file_never_does() {
    use std::os::unix::fs::MetadataExt as _;
    let identity_of = |path: &Path| {
        let meta = std::fs::metadata(path).unwrap();
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            meta.dev(),
            meta.ino(),
            meta.size(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.ctime(),
            meta.ctime_nsec()
        )
    };
    let base = scratch("hazard");
    let archive = base.join("Options");
    let year = archive.join("2024.zip");
    put(&year, b"invented year zip, 2024");
    let queue = base.join("options-queue");
    std::fs::create_dir_all(&queue).unwrap();
    std::fs::hard_link(&year, queue.join("2024.preserved.1.zip.part")).unwrap();
    let ticks = base.join("tickstore");
    let tick = ticks.join("cm/d.bts");
    put(&tick, b"an invented tick day");
    std::fs::hard_link(&tick, ticks.join("cm/d.again.bts")).unwrap();
    // Each identity is read AFTER its link, since a link changes the ctime.
    let proof = base.join("options-nested-proof");
    put(
        &proof.join("2024/VERIFIED.tsv"),
        format!("day\tsource_identity\n2024-01-01\t{}\n", identity_of(&year)).as_bytes(),
    );
    let scans = base.join("scans");
    put(
        &scans.join("2024/VERIFIED.tsv"),
        format!("day\tsource_identity\n2024-01-01\t{}\n", identity_of(&tick)).as_bytes(),
    );
    let register = host_walk(&config_of(
        &[&queue, &proof, &scans],
        vec![
            live(LiveKind::NfoArchives, &archive),
            live(LiveKind::TickStore, &ticks),
        ],
    ));
    assert_eq!(
        register
            .hazards
            .iter()
            .map(|hazard| (&hazard.path, &hazard.live))
            .collect::<Vec<_>>(),
        vec![(&queue.join("2024.preserved.1.zip.part"), &year)],
        "the queue's name is red; the tick store's own two names are not"
    );
    assert_eq!(register.roots[1].twin, Twin::Proven { sources: 1 });
    let Twin::NotProven(why) = &register.roots[2].twin else {
        panic!("a tick file is no archive: {:?}", register.roots[2].twin);
    };
    assert!(why.contains("0 of 1 source rows match"), "{why}");
    let _ = std::fs::remove_dir_all(&base);
}

/// A FILE WITH ONE NAME IS HELD AS A COUNT, never by its path: the walk keeps
/// O(1) memory per such file (a tick store's millions of files cost one sum),
/// and only a file with a second name is held with its names, for the merge.
#[test]
fn a_file_with_one_name_is_counted_and_only_a_shared_file_is_held_by_name() {
    let fs = Fake::default();
    fs.dir("/copy", 1, 1);
    fs.file("/copy/a", 1, 2, 1, b"abc");
    fs.file("/copy/b", 1, 3, 1, b"de");
    fs.file("/copy/c", 1, 4, 2, b"fghi");
    let walked = walk_root(&fs, Path::new("/copy"), &HashSet::new(), false).unwrap();
    assert_eq!((walked.files, walked.single_bytes), (3, 5));
    assert_eq!(
        walked.shared.keys().collect::<Vec<_>>(),
        vec![&(1, 4)],
        "only the file with a second name is held by name"
    );
    assert!(
        walked.identities.is_empty(),
        "a copy root keeps no identity"
    );
}

/// THE MERGE'S TABLES ARE SIZED ONCE from the walks' own counts, so no insert
/// while the walks are merged rehashes (the pre-sized sets of `CLAUDE.md` §3
/// rule 4): the archive table at what it is asked for, and the name tables at
/// every file the live walks hold.
#[test]
fn the_merge_tables_are_sized_once_from_the_walks_counts() {
    assert!(Archives::with_capacity(64).by_ino_size.capacity() >= 64);
    let mut walked = Walked::default();
    for ino in 0..40 {
        walked.identities.push((
            PathBuf::from(format!("/arch/{ino}.zip")),
            meta(Kind::File, 1, ino, 1, 4),
        ));
    }
    let config = config_of(&[], vec![live(LiveKind::NfoArchives, Path::new("/arch"))]);
    let (reports, links) = live_side(&config, vec![Some(Ok(walked))]);
    assert_eq!(reports.len(), 1);
    assert!(links.live_names.capacity() >= 40);
    assert!(links.all_names.capacity() >= 40);
    assert!(links.counted_in.capacity() >= 40);
    assert!(links.archives.by_ino_size.capacity() >= 40);
}

/// The register never removes, moves, renames, creates or writes a file:
/// every touch of the disk is one of the seam's three reads, and the tree,
/// hazard names and receipts included, is the same after a walk, a
/// retirement list and a cached answer as before (D-7047).
#[test]
fn the_register_never_removes_or_renames_a_file() {
    /// The host, counting each kind of call.
    #[derive(Default)]
    struct Counting {
        lstats: AtomicUsize,
        lists: AtomicUsize,
        reads: AtomicUsize,
    }
    impl Fs for Counting {
        fn lstat(&self, path: &Path) -> io::Result<Meta> {
            self.lstats.fetch_add(1, Ordering::SeqCst);
            Host.lstat(path)
        }
        fn list(&self, dir: &Path, each: &mut dyn FnMut(OsString)) -> io::Result<()> {
            self.lists.fetch_add(1, Ordering::SeqCst);
            Host.list(dir, each)
        }
        fn lines(&self, path: &Path, each: &mut dyn FnMut(&[u8])) -> Result<Ending, String> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Host.lines(path, each)
        }
    }
    let base = scratch("never-removes");
    let archive = base.join("Options");
    let queue = base.join("queue");
    put(&archive.join("2022.zip"), b"invented");
    std::fs::create_dir_all(&queue).unwrap();
    std::fs::hard_link(archive.join("2022.zip"), queue.join("2022.x.zip.part")).unwrap();
    put(&queue.join("run/ALL_VERIFIED.txt"), b"proved\n");
    put(&queue.join("2022.zip.tmp"), b"partial");
    let before = snapshot(&base);
    let fs = Counting::default();
    let config = config_of(&[&queue], vec![live(LiveKind::NfoArchives, &archive)]);
    let cache = Cache::new();
    for _ in 0..2 {
        let Served::Register { register, .. } = answer(&cache, &fs, &config, &|| 1) else {
            panic!("a register");
        };
        let _ = retirement_text(&register);
        note_walked(&register);
    }
    assert_eq!(snapshot(&base), before, "nothing under the roots changed");
    assert!(fs.lstats.load(Ordering::SeqCst) > 0);
    assert!(fs.lists.load(Ordering::SeqCst) > 0);
    assert_eq!(fs.reads.load(Ordering::SeqCst), 1, "one receipt, read once");
    // AND STRUCTURALLY: the module's own code names no call that removes,
    // moves, renames, truncates or creates; R4-06's scan holds the same
    // rule over this file (`crates/pull/tests/no_scheduler_delete.rs`).
    let source = include_str!("copies_register.rs");
    let production = source.split("#[cfg(test)]").next().unwrap();
    for word in [
        "remove_file",
        "remove_dir",
        "fs::rename",
        "hard_link(",
        "File::create",
        "create_dir",
        "OpenOptions",
        "set_len",
        "fs::write",
        "fs::copy",
    ] {
        assert!(!production.contains(word), "the register names {word}");
    }
    let _ = std::fs::remove_dir_all(&base);
}

/// An unset `disk.extra_copy_roots` is a named wait: no root is walked, no
/// default folder is put in, and the wait names the key; a stored value that
/// does not read is a wait naming why, never a guess (D-7040).
#[test]
fn an_unset_extra_copy_roots_setting_is_a_named_wait_not_a_default() {
    let fs = Fake::default();
    let env = |_: &str| -> Option<OsString> { None };
    let sources = Sources {
        copy_roots: None,
        source_gdfl: Some("/invented/gdfl"),
        source_masters: None,
        store_root: Path::new("/invented/store"),
        env: &env,
    };
    let wait = config(&sources).unwrap_err();
    assert_eq!(wait, Wait::Unset);
    let words = wait.to_string();
    assert!(words.contains(KEY), "{words}");
    assert!(words.contains("never a default"), "{words}");
    assert!(words.contains("nothing is walked"), "{words}");
    assert_eq!(fs.lstats.load(Ordering::SeqCst), 0);
    assert_eq!(fs.lists.load(Ordering::SeqCst), 0);
    assert!(matches!(total(&Cache::new()), Total::NotWalked));
    for bad in ["relative/folder", "/a|/a", "/a||/b", "/a|/a/b"] {
        let sources = Sources {
            copy_roots: Some(bad),
            ..sources
        };
        match config(&sources) {
            Err(Wait::Invalid(why)) => {
                assert!(!why.is_empty());
                assert!(Wait::Invalid(why).to_string().contains("nothing is walked"));
            }
            other => panic!("{bad:?} must be a named wait, got {other:?}"),
        }
    }
}

/// The retirement list is text for the operator with no command in it: no
/// word of a command that deletes, moves or renames outside the quoted
/// paths, even when a folder's own name holds such words; the fixed
/// sentence heads it (D-7047; FM-607 "done" 3).
#[test]
fn the_retirement_list_contains_no_delete_command() {
    let base = scratch("no-command");
    let archive = base.join("Options");
    let copy = base.join("rm -rf delete; mv unlink");
    put(&archive.join("2023.zip"), b"invented");
    put(&copy.join("a.csv"), b"invented rows");
    std::fs::hard_link(archive.join("2023.zip"), copy.join("2023.x.zip.part")).unwrap();
    let register = host_walk(&config_of(
        &[&copy],
        vec![live(LiveKind::NfoArchives, &archive)],
    ));
    let text = retirement_text(&register);
    assert!(text.contains(RETIREMENT_SENTENCE), "{text}");
    assert!(text.contains("[HAZARD]"), "{text}");
    assert_eq!(retirement(&register).len(), 2);
    let outside_quotes = unquoted(&text).to_lowercase();
    for word in COMMAND_WORDS {
        assert!(
            !outside_quotes.contains(word),
            "the list says {word:?}:\n{text}"
        );
    }
    assert!(
        !outside_quotes.contains('`') && !outside_quotes.contains("$("),
        "{text}"
    );
    // AND THE CHECK ITSELF CAN FAIL: the blanked text of a line holding a
    // command is caught.
    assert!(unquoted("then rm \"x\"").contains("rm "));
    let _ = std::fs::remove_dir_all(&base);
}

/// The same inode number on another device is another file: within one walk
/// a hard link is the same `(st_dev, st_ino)`, never the inode alone
/// (amendment (e); D-7042). The same pair on the same device is a link.
#[test]
fn the_same_inode_on_another_device_is_not_a_link() {
    for (copy_dev, linked) in [(2_u64, false), (1, true)] {
        let fs = Fake::default();
        fs.dir("/archive", 1, 10);
        fs.file("/archive/2020.zip", 1, 42, 2, b"invented");
        fs.dir("/copies", copy_dev, 11);
        fs.file("/copies/2020.x.zip.part", copy_dev, 42, 2, b"invented");
        let config = config_of(
            &[Path::new("/copies")],
            vec![live(LiveKind::NfoArchives, Path::new("/archive"))],
        );
        let register = walk(&fs, &config, 0);
        assert_eq!(
            register.hazards.len(),
            usize::from(linked),
            "dev {copy_dev}"
        );
        let root = &register.roots[0];
        if linked {
            assert_eq!(root.extra_bytes, 0, "the bytes are the archive's");
            assert!(root.linked_elsewhere.is_empty());
        } else {
            assert_eq!(root.extra_bytes, 8);
            assert_eq!(
                root.linked_elsewhere.iter().collect::<Vec<_>>(),
                vec![Path::new("/copies/2020.x.zip.part")],
                "its other name is outside every walked root"
            );
        }
    }
}

/// A served answer names when it was walked and how old the walk is; a
/// second request is served from memory with the same time; a link made
/// deep in the tree does not change the root's own stat, so the cached
/// answer still names its old walk (amendment (f)); a new direct entry does,
/// and the next request walks again and says why (D-7046).
#[test]
fn a_cached_answer_names_its_walk_time() {
    let base = scratch("cache");
    let copy = base.join("copy");
    put(&copy.join("deep/inside/a.csv"), b"invented");
    let config = config_of(&[&copy], Vec::new());
    let cache = Cache::new();
    let clock = AtomicU64::new(1_000);
    let now = || i64::try_from(clock.fetch_add(1, Ordering::SeqCst)).unwrap();
    let Served::Register {
        register: first,
        fresh: true,
        age_ms: 0,
        why,
        rewalking: false,
    } = answer(&cache, &Host, &config, &now)
    else {
        panic!("the first answer walks");
    };
    assert_eq!(first.walked_at_micros, 1_000);
    assert_eq!(why, "the first request since start");
    std::thread::sleep(std::time::Duration::from_millis(5));
    let Served::Register {
        register: second,
        fresh: false,
        age_ms,
        ..
    } = answer(&cache, &Host, &config, &now)
    else {
        panic!("the second answer is held");
    };
    assert_eq!(second.walked_at_micros, 1_000, "the same walk");
    assert!(age_ms >= 5, "{age_ms}");
    // A LINK MADE DEEP IN THE TREE: the root's own stat does not move.
    std::fs::hard_link(
        copy.join("deep/inside/a.csv"),
        copy.join("deep/inside/b.csv"),
    )
    .unwrap();
    let Served::Register {
        register: third,
        fresh: false,
        ..
    } = answer(&cache, &Host, &config, &now)
    else {
        panic!("still held");
    };
    assert_eq!(
        third.walked_at_micros, 1_000,
        "the answer names its old walk"
    );
    assert!(
        third.roots[0].linked_elsewhere.is_empty(),
        "the deep link is not seen yet"
    );
    // A DIRECT ENTRY: the root's own stat moves, and the next request walks.
    put(&copy.join("new.csv"), b"x");
    let Served::Register {
        register: fourth,
        fresh: true,
        why,
        ..
    } = answer(&cache, &Host, &config, &now)
    else {
        panic!("a fresh walk");
    };
    assert!(fourth.walked_at_micros > 1_000);
    assert!(why.contains("own stat"), "{why}");
    assert_eq!(fourth.roots[0].files, 3);
    match total(&cache) {
        Total::Measured {
            bytes,
            walked_at_micros,
            whole: true,
        } => {
            assert_eq!(bytes, fourth.extra_bytes);
            assert_eq!(walked_at_micros, fourth.walked_at_micros);
        }
        other => panic!("{other:?}"),
    }
    let _ = std::fs::remove_dir_all(&base);
}

// ═══ Extreme cases ═════════════════════════════════════════════════════════

/// EMPTY AND ONE-FILE ROOTS: an empty folder is walked whole with nothing in
/// it and offered for retirement freeing 0 bytes; a root of one file is
/// exactly that file.
#[test]
fn an_empty_root_and_a_one_file_root_are_counted_exactly() {
    let base = scratch("empty-one");
    let empty = base.join("empty");
    let one = base.join("one");
    std::fs::create_dir_all(&empty).unwrap();
    put(&one.join("only.bin"), &[0_u8; 4_096]);
    let register = host_walk(&config_of(&[&empty, &one], Vec::new()));
    assert_eq!(
        (
            register.roots[0].files,
            register.roots[0].bytes,
            register.roots[0].newest_mtime_s
        ),
        (0, 0, None)
    );
    assert_eq!(
        (register.roots[1].files, register.roots[1].bytes),
        (1, 4_096)
    );
    assert_eq!(register.extra_bytes, 4_096);
    assert!(register.whole());
    let list = retirement(&register);
    assert_eq!(
        list.iter().map(|r| r.bytes).collect::<Vec<_>>(),
        vec![0, 4_096]
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// A ROOT THAT IS GONE, A FILE, OR A LINK is refused by name with no figure;
/// a name that vanishes between the listing and its look is a named fault,
/// the root is not walked whole, it is kept off the retirement list, and a
/// `warn` event names it for `/logs`.
#[test]
fn a_root_that_vanishes_or_is_not_a_folder_is_refused_by_name() {
    let base = scratch("vanish");
    let file = base.join("a-file");
    put(&file, b"x");
    let target = base.join("target");
    std::fs::create_dir_all(&target).unwrap();
    let link = base.join("a-link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let gone = base.join("gone");
    let register = host_walk(&config_of(&[&gone, &file, &link], Vec::new()));
    let why: Vec<String> = register
        .roots
        .iter()
        .map(|root| match &root.state {
            State::Refused(why) => why.clone(),
            other @ State::Walked { .. } => panic!("{other:?}"),
        })
        .collect();
    assert!(why[0].contains("No such file"), "{why:?}");
    assert_eq!(why[1], "not a directory");
    assert!(why[2].contains("symbolic link"), "{why:?}");
    assert!(retirement(&register).is_empty());
    assert!(!register.whole());
    let warns = notes(&register)
        .into_iter()
        .filter(|note| note.level == telemetry::Level::Warn)
        .count();
    assert_eq!(warns, 3);
    let fs = Fake::default();
    fs.dir("/copy", 1, 1);
    fs.file("/copy/kept", 1, 2, 1, b"abc");
    fs.vanished("/copy", "went-away");
    let register = walk(&fs, &config_of(&[Path::new("/copy")], Vec::new()), 0);
    let root = &register.roots[0];
    assert_eq!(root.state, State::Walked { whole: false });
    assert_eq!(
        root.faults.iter().next().unwrap(),
        &(
            PathBuf::from("/copy/went-away"),
            "vanished while the walk ran".to_owned()
        )
    );
    assert_eq!(root.bytes, 3, "what was read is still counted");
    assert!(
        retirement(&register).is_empty(),
        "never offered when not whole"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// A LINK LOOP AND A FOLDER REACHED BY TWO NAMES: links are counted and
/// never followed, so a loop ends; a folder met again by its `(st_dev,
/// st_ino)` is walked once and named, so nothing is counted twice.
#[test]
fn a_link_loop_and_a_folder_met_twice_are_walked_once() {
    let base = scratch("loop");
    let copy = base.join("copy");
    put(&copy.join("a/x.bin"), &[1_u8; 10]);
    std::os::unix::fs::symlink(&copy, copy.join("a/loop")).unwrap();
    std::os::unix::fs::symlink(copy.join("a"), copy.join("back")).unwrap();
    let register = host_walk(&config_of(&[&copy], Vec::new()));
    let root = &register.roots[0];
    assert_eq!((root.files, root.links, root.bytes), (1, 2, 10));
    let fs = Fake::default();
    fs.dir("/copy", 1, 1);
    fs.dir("/copy/a", 1, 2);
    fs.file("/copy/a/x", 1, 3, 1, b"abcd");
    fs.second_name_of_dir("/copy", "mount-again", "/copy/a");
    fs.second_name_of_dir("/copy/a", "up", "/copy");
    let register = walk(&fs, &config_of(&[Path::new("/copy")], Vec::new()), 0);
    let root = &register.roots[0];
    assert_eq!((root.files, root.bytes), (1, 4));
    assert_eq!(root.revisited.len(), 2, "{:?}", root.revisited);
    assert_eq!(root.state, State::Walked { whole: true });
    let _ = std::fs::remove_dir_all(&base);
}

/// A NAME WITH A COMMA, A LINE BREAK, OR A BYTE THAT IS NOT UTF-8: each is
/// counted, and the retirement list writes each path exactly, escaped, on
/// one line (out of the box: the 13 Oct tree held none, and a raw line
/// break would split a path in two for the reader).
#[test]
fn names_with_a_comma_a_line_break_or_no_utf8_stay_one_line() {
    use std::os::unix::ffi::OsStrExt as _;
    let base = scratch("names");
    let archive = base.join("Options");
    let copy = base.join("copy, with a comma");
    put(&archive.join("2025.zip"), b"invented");
    put(&copy.join("a,b.csv"), b"1");
    put(&copy.join("line\nbreak.csv"), b"22");
    let odd = copy.join(OsStr::from_bytes(b"bad\xff.zip.part"));
    std::fs::hard_link(archive.join("2025.zip"), &odd).unwrap();
    let register = host_walk(&config_of(
        &[&copy],
        vec![live(LiveKind::NfoArchives, &archive)],
    ));
    assert_eq!(register.roots[0].files, 3);
    assert_eq!(register.hazards.first().unwrap().path, odd);
    let text = retirement_text(&register);
    for line in text.lines() {
        assert!(!line.starts_with("break"), "a path was split:\n{text}");
    }
    assert!(
        text.contains(r"\xFF") || text.contains(r"\u{fffd}") || text.contains(r"\xff"),
        "{text}"
    );
    assert!(quoted(&copy.join("line\nbreak.csv")).contains(r"\n"));
    let _ = std::fs::remove_dir_all(&base);
}

/// AN UNREADABLE SUBFOLDER is a named fault: the rest of the root is read,
/// the root is not walked whole, it is kept off the retirement list, and a
/// `warn` names it.
#[test]
fn an_unreadable_subfolder_is_a_named_fault() {
    let fs = Fake::default();
    fs.dir("/copy", 1, 1);
    fs.dir("/copy/locked", 1, 2);
    fs.file("/copy/open.csv", 1, 3, 1, b"12345");
    fs.refuse_listing("/copy/locked");
    let register = walk(&fs, &config_of(&[Path::new("/copy")], Vec::new()), 0);
    let root = &register.roots[0];
    assert_eq!(root.state, State::Walked { whole: false });
    let (path, why) = root.faults.iter().next().unwrap();
    assert_eq!(path, Path::new("/copy/locked"));
    assert!(why.contains("cannot be listed"), "{why}");
    assert_eq!(root.bytes, 5);
    assert!(retirement(&register).is_empty());
    assert!(
        notes(&register)
            .iter()
            .any(|note| note.level == telemetry::Level::Warn
                && note.fields.iter().any(|(_, v)| v.contains("1 fault")))
    );
    // THE ROOT ITSELF REFUSING ITS LISTING is no walk at all.
    fs.refuse_listing("/copy");
    let register = walk(&fs, &config_of(&[Path::new("/copy")], Vec::new()), 0);
    assert!(
        matches!(&register.roots[0].state, State::Refused(why) if why.contains("cannot be listed"))
    );
}

/// RECEIPTS THAT PROVE NOTHING: empty, white space only, torn, not UTF-8,
/// and a `.prove.out` without `ALL-PROVED` are each named, none is a proof,
/// and a `.prove.out` that says `ALL-PROVED` is one.
#[test]
fn a_torn_empty_or_silent_receipt_is_no_proof() {
    let fs = Fake::default();
    fs.dir("/copy", 1, 1);
    let cases: [(&str, &[u8], &str); 7] = [
        ("/copy/e/ALL_VERIFIED.txt", b"", "empty"),
        ("/copy/w/ALL_VERIFIED.txt", b"  \n\t\n", "empty"),
        ("/copy/t/ALL_VERIFIED.txt", b"every archive passed", "torn"),
        ("/copy/u/ALL_VERIFIED.txt", b"ok\n\xff\xfe\n", "unreadable"),
        (
            "/copy/logs/a.prove.out",
            b"checked 12 entries\n",
            "no_claim",
        ),
        (
            "/copy/logs/b.prove.out",
            b"checked 12 entries\nALL-PROVED\n",
            "states",
        ),
        (
            "/copy/v/VERIFIED.tsv",
            b"day\tcrc\n2020-01-01\tabc\n",
            "states",
        ),
    ];
    for (n, (path, bytes, _)) in cases.iter().enumerate() {
        let dir = Path::new(path).parent().unwrap().to_str().unwrap();
        if fs.nodes.lock().unwrap().get(Path::new(dir)).is_none() {
            fs.dir(dir, 1, 100 + n as u64);
        }
        fs.file(path, 1, 200 + n as u64, 1, bytes);
    }
    // NOT A RECEIPT: the bare suffix, and a name that only ends like one.
    fs.file("/copy/.prove.out", 1, 300, 1, b"ALL-PROVED\n");
    fs.file("/copy/NOT_ALL_VERIFIED.txt", 1, 301, 1, b"x\n");
    let register = walk(&fs, &config_of(&[Path::new("/copy")], Vec::new()), 0);
    let root = &register.roots[0];
    assert_eq!(root.receipt_count, 7);
    for (path, _, verdict) in cases {
        let receipt = root
            .receipts
            .iter()
            .find(|r| r.path == Path::new(path))
            .unwrap();
        assert_eq!(receipt.verdict, verdict, "{path}");
        assert_eq!(receipt.why.is_none(), verdict == "states", "{path}");
    }
    assert_eq!(
        root.proof.as_deref(),
        Some(Path::new("/copy/logs/b.prove.out")),
        "the first stating receipt by path"
    );
    match &root.twin {
        Twin::NotProven(why) => assert!(why.contains("names no source_identity row"), "{why}"),
        other @ Twin::Proven { .. } => panic!("{other:?}"),
    }
}

/// A RECEIPT WRITTEN BEFORE A REMOUNT: its `source_identity` records another
/// `st_dev` while inode, size and both times match the archive today, and it
/// still proves the twin; a row whose size differs does not, and the twin
/// is not proven with the row named (13 Oct re-measure; D-7045).
#[test]
fn a_receipt_from_before_a_remount_still_matches_by_inode_size_and_times() {
    let fs = Fake::default();
    fs.dir("/Options", 7, 1);
    fs.file("/Options/2020.zip", 7, 4242, 1, b"invented archive");
    fs.dir("/proof", 7, 2);
    let remounted = "99:4242:16:1700000000:7:1700000001:9";
    fs.file(
        "/proof/VERIFIED.tsv",
        7,
        3,
        1,
        format!("name\tsource_identity\na\t{remounted}\n").as_bytes(),
    );
    let config = config_of(
        &[Path::new("/proof")],
        vec![live(LiveKind::NfoArchives, Path::new("/Options"))],
    );
    let register = walk(&fs, &config, 0);
    assert_eq!(register.roots[0].twin, Twin::Proven { sources: 1 });
    fs.dir("/proof2", 7, 4);
    fs.file(
        "/proof2/VERIFIED.tsv",
        7,
        5,
        1,
        b"name\tsource_identity\na\t99:4242:17:1700000000:7:1700000001:9\nb\tnot-an-identity\n",
    );
    let config = config_of(
        &[Path::new("/proof2")],
        vec![live(LiveKind::NfoArchives, Path::new("/Options"))],
    );
    let register = walk(&fs, &config, 0);
    let receipt = &register.roots[0].receipts[0];
    assert_eq!((receipt.sources, receipt.matched), (2, 0));
    assert!(
        receipt
            .first_unmatched
            .as_deref()
            .unwrap()
            .starts_with("row 2:")
    );
    match &register.roots[0].twin {
        Twin::NotProven(why) => assert!(why.contains("0 of 2"), "{why}"),
        other @ Twin::Proven { .. } => panic!("{other:?}"),
    }
}

/// A COPY ROOT INSIDE A LIVE ROOT (the 13 Oct `prove-20261004` inside the
/// tick store's folder): outside the tick store's read tops it has no reader
/// and its retirement line names the live root it lies in; inside `cm/` it
/// has the tick store's reader and is never offered (D-7044).
#[test]
fn a_copy_root_inside_a_live_root_is_read_only_where_that_root_reads() {
    let base = scratch("inside");
    let ticks = base.join("tickstore-data");
    let proof = ticks.join("prove-20261004");
    let inner = ticks.join("cm/2024");
    put(&proof.join("all-proven.txt"), b"invented list\n");
    put(&inner.join("d.bts"), b"invented day");
    let register = host_walk(&config_of(
        &[&proof, &inner],
        vec![live(LiveKind::TickStore, &ticks)],
    ));
    let (outside, read) = (&register.roots[0], &register.roots[1]);
    assert_eq!(outside.reader, None);
    assert_eq!(outside.inside.as_ref().unwrap().kind, LiveKind::TickStore);
    assert_eq!(read.reader.as_ref().unwrap().kind, LiveKind::TickStore);
    let list = retirement(&register);
    assert_eq!(list.len(), 1);
    assert!(
        list[0]
            .caution
            .as_deref()
            .unwrap()
            .contains("never its parent")
    );
    // THE TICK STORE'S WALK SKIPS BOTH COPY ROOTS: each file is one root's.
    let live = register
        .live
        .iter()
        .find(|l| l.root.kind == LiveKind::TickStore)
        .unwrap();
    assert_eq!(live.files, 0, "the copy roots are walked as copies only");
    let _ = std::fs::remove_dir_all(&base);
}

/// THE READER RULE ON EVERY KIND: a copy root that IS a live root, or holds
/// one, has its reader and is never offered; inside an archive, the masters
/// or the store it has that root's reader; in the ledger only `<yyyy>/_v3`
/// (and a `<yyyy>` that holds it) is read (D-7044).
#[test]
fn the_reader_rule_names_each_kind() {
    let lives = vec![
        live(LiveKind::NfoArchives, Path::new("/v/Options")),
        live(LiveKind::V3Ledger, Path::new("/v/NSE_Options_Tick")),
        live(LiveKind::Store, Path::new("/v/store")),
        live(LiveKind::SourceMasters, Path::new("/v/masters")),
    ];
    let cases: [(&str, Option<LiveKind>, Option<LiveKind>, &str); 9] = [
        ("/v/Options", Some(LiveKind::NfoArchives), None, "IS"),
        ("/v", Some(LiveKind::NfoArchives), None, "holds"),
        (
            "/v/Options/2020",
            Some(LiveKind::NfoArchives),
            None,
            "reads",
        ),
        (
            "/v/NSE_Options_Tick/2026/_v3",
            Some(LiveKind::V3Ledger),
            None,
            "reads",
        ),
        (
            "/v/NSE_Options_Tick/2026",
            Some(LiveKind::V3Ledger),
            None,
            "reads",
        ),
        (
            "/v/NSE_Options_Tick/2026/ticks",
            None,
            Some(LiveKind::V3Ledger),
            "",
        ),
        (
            "/v/NSE_Options_Tick/scans",
            None,
            Some(LiveKind::V3Ledger),
            "",
        ),
        ("/v/store/bars", Some(LiveKind::Store), None, "reads"),
        ("/w/elsewhere", None, None, ""),
    ];
    for (copy, reader, inside, how) in cases {
        let (got, within) = reader_of(Path::new(copy), &lives);
        assert_eq!(got.as_ref().map(|r| r.kind), reader, "{copy}");
        assert_eq!(within.map(|l| l.kind), inside, "{copy}");
        if let Some(got) = got {
            assert!(got.how.contains(how), "{copy}: {}", got.how);
        }
    }
    assert!(!LiveKind::V3Ledger.reads(Path::new("20x6")));
    assert!(LiveKind::TickStore.reads(Path::new("options/2020")));
    assert!(!LiveKind::TickStore.reads(Path::new("FORMAT.md")));
}

/// THE TICK STORE'S READ TOPS ARE ITS READERS' OWN: the two lines that walk
/// `cm` and `options` are in the readers' source, so a reader that grows a
/// top without this list fails here.
#[test]
fn tick_store_tops_are_the_readers_own() {
    let cm = include_str!("gdfl_tickstore.rs");
    let nfo = include_str!("gdfl_nfo.rs");
    assert!(cm.contains("DayTable::walk(&self.root, \"cm\""));
    assert!(nfo.contains("DayTable::walk(&self.root, \"options\""));
    assert_eq!(TICK_STORE_READ_TOPS, ["cm", "options"]);
    for source in [cm, nfo] {
        assert_eq!(
            source.matches("DayTable::walk(&self.root,").count(),
            1,
            "one top per reader"
        );
    }
}

/// COMBINATION: a `.tmp` name linked to the CM archive, a name linked to the
/// tick store, a file shared by two copy roots and a file linked outside
/// every walked root, in one walk: two red rows of two kinds, the shared
/// file's bytes counted once (in the first root), and the outside link named.
#[test]
fn hazards_of_two_kinds_a_shared_file_and_an_outside_link_in_one_walk() {
    let base = scratch("combo");
    let cm = base.join("cm-archive");
    let ticks = base.join("tickstore");
    let outside = base.join("not-configured");
    let one = base.join("one");
    let two = base.join("two");
    put(&cm.join("GFDLCM_INVENTED.zip"), b"cm bytes");
    put(&ticks.join("cm/d.bts"), b"tick bytes");
    put(&outside.join("o.bin"), b"outside!");
    put(&one.join("shared.bin"), &[5_u8; 100]);
    std::fs::create_dir_all(&two).unwrap();
    std::fs::hard_link(cm.join("GFDLCM_INVENTED.zip"), one.join("day.zip.tmp")).unwrap();
    std::fs::hard_link(ticks.join("cm/d.bts"), two.join("d.bts")).unwrap();
    std::fs::hard_link(one.join("shared.bin"), two.join("shared-again.bin")).unwrap();
    std::fs::hard_link(outside.join("o.bin"), two.join("o-again.bin")).unwrap();
    let register = host_walk(&config_of(
        &[&one, &two],
        vec![
            live(LiveKind::CmArchive, &cm),
            live(LiveKind::TickStore, &ticks),
        ],
    ));
    let kinds: Vec<(LiveKind, bool)> = register
        .hazards
        .iter()
        .map(|h| (h.kind, h.partial))
        .collect();
    assert_eq!(kinds.len(), 2, "{:?}", register.hazards);
    assert!(kinds.contains(&(LiveKind::CmArchive, true)));
    assert!(kinds.contains(&(LiveKind::TickStore, false)));
    assert_eq!(register.roots[0].extra_bytes, 100, "the shared file, once");
    assert_eq!(
        register.roots[1].extra_bytes, 8,
        "only the outside link's bytes"
    );
    assert_eq!(register.extra_bytes, 108);
    assert_eq!(
        register.roots[1]
            .linked_elsewhere
            .iter()
            .collect::<Vec<_>>(),
        vec![&two.join("o-again.bin")]
    );
    assert!(
        retirement(&register)[1]
            .caution
            .as_deref()
            .unwrap()
            .contains("another name outside")
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// COMBINATION: a copy root that IS the archive root, and one that HOLDS
/// the tick store: neither is offered, the archive's own names are never
/// red against themselves, and the held tick store is skipped and named
/// where it lies, never counted as a copy.
#[test]
fn a_copy_root_that_is_or_holds_a_live_root_is_never_offered() {
    let base = scratch("is-holds");
    let archive = base.join("Options");
    let around = base.join("around");
    let ticks = around.join("tickstore");
    put(&archive.join("2020.zip"), b"year");
    std::fs::hard_link(archive.join("2020.zip"), archive.join("2020.again.zip")).unwrap();
    put(&ticks.join("cm/d.bts"), b"tick");
    put(&around.join("loose.csv"), b"loose");
    let register = host_walk(&config_of(
        &[&archive, &around],
        vec![
            live(LiveKind::NfoArchives, &archive),
            live(LiveKind::TickStore, &ticks),
        ],
    ));
    assert!(register.hazards.is_empty(), "{:?}", register.hazards);
    assert!(
        register.roots[0]
            .reader
            .as_ref()
            .unwrap()
            .how
            .contains("IS")
    );
    assert!(
        register.roots[1]
            .reader
            .as_ref()
            .unwrap()
            .how
            .contains("holds")
    );
    assert_eq!(
        register.roots[1].skipped.iter().collect::<Vec<_>>(),
        vec![&ticks]
    );
    assert_eq!(
        register.roots[1].bytes, 5,
        "the tick store is not counted as a copy"
    );
    assert!(retirement(&register).is_empty());
    let _ = std::fs::remove_dir_all(&base);
}

/// TWENTY REQUESTS DURING A COLD WALK: exactly one walk runs; every other
/// request is answered at once, never made to wait, as `Walking`; once it
/// lands they are served the held register (out of the box: a page polled by
/// many tabs at start).
#[test]
fn twenty_requests_during_a_cold_walk_take_one_walk() {
    let fs = std::sync::Arc::new(Fake::default());
    fs.dir("/copy", 1, 1);
    fs.file("/copy/a", 1, 2, 1, b"abc");
    *fs.hold.lock().unwrap() = Some(PathBuf::from("/copy/a"));
    fs.holding.store(true, Ordering::SeqCst);
    let config = config_of(&[Path::new("/copy")], Vec::new());
    let cache = std::sync::Arc::new(Cache::new());
    let first = {
        let (fs, cache, config) = (fs.clone(), cache.clone(), config.clone());
        std::thread::spawn(move || answer(&cache, fs.as_ref(), &config, &|| 5))
    };
    // A walk that ends without reaching the held file is a failure here, at
    // once, never a wait that lasts for ever.
    while !fs.held.load(Ordering::SeqCst) {
        assert!(
            !first.is_finished(),
            "the first request's walk ended without reaching /copy/a"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let others: Vec<_> = (0..20)
        .map(|_| {
            let (fs, cache, config) = (fs.clone(), cache.clone(), config.clone());
            std::thread::spawn(move || answer(&cache, fs.as_ref(), &config, &|| 6))
        })
        .collect();
    for other in others {
        assert_eq!(other.join().unwrap(), Served::Walking);
    }
    fs.holding.store(false, Ordering::SeqCst);
    let Served::Register { fresh: true, .. } = first.join().unwrap() else {
        panic!("the first request walked");
    };
    assert_eq!(fs.lists.load(Ordering::SeqCst), 1, "one walk");
    let Served::Register {
        fresh: false,
        register,
        ..
    } = answer(&cache, fs.as_ref(), &config, &|| 7)
    else {
        panic!("held");
    };
    assert_eq!(register.walked_at_micros, 5);
    assert_eq!(fs.lists.load(Ordering::SeqCst), 1, "still one walk");
}

/// A PIPE NAMED LIKE A RECEIPT is never opened (out of the box: a FIFO at a
/// receipt's name made an earlier reader wait for ever); a regular receipt
/// beside it is read once.
#[test]
fn a_pipe_named_like_a_receipt_is_never_opened() {
    let fs = Fake::default();
    fs.dir("/copy", 1, 1);
    fs.add(
        "/copy/VERIFIED.tsv",
        Node {
            meta: meta(Kind::Other, 1, 2, 1, 0),
            children: None,
            bytes: Vec::new(),
        },
    );
    fs.dir("/copy/run", 1, 3);
    fs.file("/copy/run/ALL_VERIFIED.txt", 1, 4, 1, b"passed\n");
    let register = walk(&fs, &config_of(&[Path::new("/copy")], Vec::new()), 0);
    assert_eq!(fs.reads.load(Ordering::SeqCst), 1);
    assert_eq!(register.roots[0].others, 1);
    assert_eq!(register.roots[0].receipt_count, 1);
    // AND ON THE HOST: `lines` goes through `store::safe_open`, which refuses
    // anything but a regular file without waiting.
    let base = scratch("pipe");
    let dir = base.join("d");
    std::fs::create_dir_all(&dir).unwrap();
    assert!(Host.lines(&dir, &mut |_| {}).is_err());
    let _ = std::fs::remove_dir_all(&base);
}

/// CLOCK EDGES: a walk time before 1970, at the epoch and at an IST day
/// boundary is written exactly; `now_micros` is never refused.
#[test]
fn walk_times_at_the_clock_edges_are_written_exactly() {
    assert_eq!(ist_text(0), "1970-01-01 05:30:00 IST");
    assert_eq!(ist_text(-1), "1970-01-01 05:29:59 IST");
    assert_eq!(ist_text(-19_800_000_001), "1969-12-31 23:59:59 IST");
    assert_eq!(ist_text(1_761_244_200_000_000), "2025-10-24 00:00:00 IST");
    assert!(now_micros() > 1_700_000_000_000_000);
}

/// THE CONFIGURATION: every live root through its one reader, unset ones
/// named, `source.masters` falling back to its variable and saying so, the
/// store root always present, in `LiveKind::ALL`'s order.
#[test]
fn the_configuration_names_every_live_root_or_why_not() {
    let env = |name: &str| -> Option<OsString> {
        match name {
            "BRUTEX_GDFL_NFO_ARCHIVES" => Some(OsString::from("/i/Options")),
            "BRUTEX_GDFL_TICKSTORE" => Some(OsString::new()),
            "BRUTEX_MASTERS" => Some(OsString::from("/i/masters")),
            _ => None,
        }
    };
    let sources = Sources {
        copy_roots: Some("/i/a|/i/b"),
        source_gdfl: None,
        source_masters: None,
        store_root: Path::new("/i/store"),
        env: &env,
    };
    let config = config(&sources).unwrap();
    assert_eq!(
        config.copy_roots,
        vec![PathBuf::from("/i/a"), PathBuf::from("/i/b")]
    );
    let kinds: Vec<LiveKind> = config.live.iter().map(|l| l.kind).collect();
    assert_eq!(
        kinds,
        vec![
            LiveKind::NfoArchives,
            LiveKind::SourceMasters,
            LiveKind::Store
        ]
    );
    assert!(config.live[1].named_by.contains("BRUTEX_MASTERS"));
    assert_eq!(config.unset_live.len(), 4, "{:?}", config.unset_live);
    assert!(
        config
            .unset_live
            .iter()
            .any(|w| w.starts_with("tick_store:"))
    );
    assert!(
        config
            .unset_live
            .iter()
            .any(|w| w.contains("source.gdfl is not set"))
    );
    let with_gdfl = config_of_sources("/i/gdfl", &env);
    assert!(
        with_gdfl
            .live
            .iter()
            .any(|l| l.kind == LiveKind::SourceGdfl && l.named_by == "source.gdfl")
    );
}

/// A configuration whose `source.gdfl` and `source.masters` are saved.
fn config_of_sources(gdfl: &str, env: &dyn Fn(&str) -> Option<OsString>) -> Config {
    config(&Sources {
        copy_roots: Some("/i/a"),
        source_gdfl: Some(gdfl),
        source_masters: Some("/i/m"),
        store_root: Path::new("/i/store"),
        env,
    })
    .unwrap()
}

/// The p50, p99 and max of `times`, ns.
fn spread(times: &mut [u128]) -> (u128, u128, u128) {
    times.sort_unstable();
    let at = |q: usize| times[(times.len() * q).div_ceil(100).max(1) - 1];
    (at(50), at(99), *times.last().unwrap())
}

/// TIMED: a held answer (n = 200 sequential calls) over copy roots of 100,
/// 1,000 and 10,000 files, a hundred to a folder, each walked once first. The
/// held answer touches no file of the tree (one `lstat` per root), so its
/// p50 at 100x is bounded against 1x. The figures for `docs/06-limits.md`
/// L-CC5-1 and L-CC5-2 are the gate-8 bench's (`crates/api/benches/ratio/
/// copies.rs`); this test prints nothing (gate 23).
#[test]
fn a_held_answer_and_a_walk_are_timed_at_1x_10x_100x() {
    let base = scratch("timed");
    let mut held_p50 = Vec::with_capacity(3);
    for files in [100_usize, 1_000, 10_000] {
        let copy = base.join(format!("copy-{files}"));
        for n in 0..files {
            put(&copy.join(format!("f{:03}/{n}.bin", n / 100)), b"invented");
        }
        let config = config_of(&[&copy], Vec::new());
        let cache = Cache::new();
        let Served::Register { register, .. } = answer(&cache, &Host, &config, &|| 0) else {
            panic!("a walk");
        };
        assert_eq!(register.roots[0].files, files as u64);
        let mut held = Vec::with_capacity(200);
        for _ in 0..200 {
            let began = std::time::Instant::now();
            let served = answer(&cache, &Host, &config, &|| 0);
            held.push(began.elapsed().as_nanos());
            assert!(matches!(served, Served::Register { fresh: false, .. }));
        }
        let (h50, _, _) = spread(&mut held);
        held_p50.push(h50);
    }
    // A HELD ANSWER DOES NOT FOLLOW THE TREE: a hundred times the files may
    // not cost ten times the p50 (a walk per request would cost a hundred).
    assert!(
        held_p50[2] < held_p50[0].saturating_mul(10).max(200_000),
        "{held_p50:?}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

// ═══ Every line and branch reached (the coverage pass of 25 Oct) ═══════════

/// A reader whose every read is refused, as a disk that fails mid-file.
struct Failing;

impl io::Read for Failing {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("an invented read error"))
    }
}

/// THE HOST'S READING, APART FROM ITS OPEN: lines are handed one at a time
/// with their break removed, a last line with no break is TORN, and a read
/// the disk refuses mid-file is that refusal in its words, never "empty".
#[test]
fn a_receipt_read_that_fails_mid_file_is_the_hosts_refusal() {
    let mut seen = Vec::new();
    let ending = lines_of(&mut &b"first\nsecond"[..], &mut |line| {
        seen.push(line.to_vec());
    });
    assert_eq!(ending, Ok(Ending::Torn));
    assert_eq!(seen, vec![b"first".to_vec(), b"second".to_vec()]);
    let ending = lines_of(&mut io::BufReader::new(Failing), &mut |_| panic!("no line"));
    assert_eq!(ending, Err("an invented read error".to_owned()));
    let base = scratch("torn-host");
    put(&base.join("ALL_VERIFIED.txt"), b"all verified, cut sh");
    let mut lines = 0;
    let ending = Host.lines(&base.join("ALL_VERIFIED.txt"), &mut |_| lines += 1);
    assert_eq!((ending, lines), (Ok(Ending::Torn), 1));
    let _ = std::fs::remove_dir_all(&base);
}

/// A SOCKET IS NEITHER A FILE, A FOLDER NOR A LINK: the host names it
/// `Other`, the walk counts it and never opens it; and a path written with a
/// `.` part still reaches the tick store's read tops by its names alone.
#[test]
fn a_socket_in_a_copy_root_is_counted_never_opened() {
    let base = scratch("socket");
    let copy = base.join("copy");
    std::fs::create_dir_all(&copy).unwrap();
    let socket = copy.join("ALL_VERIFIED.txt.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    assert_eq!(Host.lstat(&socket).unwrap().kind, Kind::Other);
    let register = host_walk(&config_of(&[&copy], Vec::new()));
    assert_eq!((register.roots[0].others, register.roots[0].files), (1, 0));
    assert!(LiveKind::TickStore.reads(Path::new("./cm/2024")));
    assert!(!LiveKind::TickStore.reads(Path::new("./prove")));
    let _ = std::fs::remove_dir_all(&base);
}

/// EVERY WAY A ROOT CAN FAIL ITS OWN LOOK is named in its stamp, and a name
/// inside a walk whose look the host refuses (not "not found") is a fault
/// of the root in the host's words.
#[test]
fn a_root_that_is_a_file_a_link_or_unreadable_is_named_in_its_stamp() {
    let fs = Fake::default();
    fs.file("/f", 1, 1, 1, b"x");
    fs.add(
        "/l",
        Node {
            meta: meta(Kind::Link, 1, 2, 1, 0),
            children: None,
            bytes: Vec::new(),
        },
    );
    fs.broken.lock().unwrap().push(PathBuf::from("/b"));
    let named = |path: &str| match root_stat(&fs, Path::new(path)) {
        RootStat::Not(why) => why,
        RootStat::Dir(stamp) => panic!("{stamp:?}"),
    };
    assert_eq!(named("/f"), "not a directory");
    assert!(named("/l").contains("symbolic link"));
    assert!(
        named("/b").starts_with("cannot be read: "),
        "{}",
        named("/b")
    );
    assert_eq!(named("/absent"), "absent: no such folder");
    fs.dir("/copy", 1, 3);
    fs.file("/copy/ok", 1, 4, 1, b"ab");
    fs.file("/copy/bad", 1, 5, 1, b"cd");
    fs.broken.lock().unwrap().push(PathBuf::from("/copy/bad"));
    let register = walk(&fs, &config_of(&[Path::new("/copy")], Vec::new()), 0);
    let root = &register.roots[0];
    assert_eq!(root.state, State::Walked { whole: false });
    let (path, why) = root.faults.iter().next().unwrap();
    assert_eq!(path, Path::new("/copy/bad"));
    assert!(why.starts_with("cannot be read: "), "{why}");
    assert_eq!(root.bytes, 2);
}

/// A LIVE ROOT NOT WALKED WHOLE is named twice over: absent, it is refused
/// with no file counted; with a folder that will not list, it names the
/// fault; each puts one `warn`, and the register is not whole, so "no
/// hazard" is never claimed over it.
#[test]
fn a_live_root_absent_or_with_a_fault_is_a_warning_and_not_whole() {
    let fs = Fake::default();
    fs.dir("/copy", 1, 1);
    fs.file("/copy/k", 1, 2, 1, b"k");
    fs.dir("/arch", 1, 3);
    fs.dir("/arch/2020", 1, 4);
    fs.file("/arch/2020/a.zip", 1, 5, 1, b"zip");
    fs.refuse_listing("/arch/2020");
    let config = config_of(
        &[Path::new("/copy")],
        vec![
            live(LiveKind::CmArchive, Path::new("/arch")),
            live(LiveKind::NfoArchives, Path::new("/missing")),
        ],
    );
    let register = walk(&fs, &config, 0);
    let faulted = register.live[0].why.as_deref().unwrap();
    assert!(faulted.starts_with("1 fault(s) while walked"), "{faulted}");
    assert!(faulted.contains("\"/arch/2020\""), "{faulted}");
    assert!(register.live[1].walked);
    assert_eq!(register.live[1].files, 0);
    assert!(
        register.live[1]
            .why
            .as_deref()
            .unwrap()
            .contains("cannot be read")
    );
    assert!(!register.whole());
    let warned: Vec<String> = notes(&register)
        .into_iter()
        .filter(|note| note.message == "live root not walked whole")
        .map(|note| note.fields[1].1.clone())
        .collect();
    assert_eq!(warned, vec!["cm_archive", "nfo_archives"]);
}

/// A `VERIFIED.tsv` that is torn, or whose `source_identity` is not an
/// identity, proves no twin and says which and why; an unreadable receipt
/// is the host's words; and an archive file with the same inode and size
/// but other times is not the source a receipt named.
#[test]
fn a_torn_or_misspelled_verified_tsv_proves_no_twin() {
    assert_eq!(ReceiptKind::AllVerified.word(), "all_verified");
    assert_eq!(ReceiptKind::VerifiedTsv.word(), "verified_tsv");
    assert_eq!(ReceiptKind::ProveOut.word(), "prove_out");
    let fs = Fake::default();
    fs.dir("/torn", 1, 1);
    fs.file(
        "/torn/VERIFIED.tsv",
        1,
        2,
        1,
        b"name\tsource_identity\na\t1:2:3:4:5:6:7",
    );
    fs.dir("/spelt", 1, 3);
    fs.file(
        "/spelt/VERIFIED.tsv",
        1,
        4,
        1,
        b"name\tsource_identity\na\tnot an identity\n",
    );
    let register = walk(
        &fs,
        &config_of(&[Path::new("/torn"), Path::new("/spelt")], Vec::new()),
        0,
    );
    let Twin::NotProven(torn) = &register.roots[0].twin else {
        panic!("{:?}", register.roots[0].twin);
    };
    assert_eq!(torn, "\"/torn/VERIFIED.tsv\" is torn");
    let receipt = &register.roots[1].receipts[0];
    assert_eq!(receipt.verdict, "states");
    assert!(
        receipt
            .first_unmatched
            .as_deref()
            .unwrap()
            .contains("\"not an identity\" is not dev:inode:size"),
        "{receipt:?}"
    );
    assert!(matches!(register.roots[1].twin, Twin::NotProven(_)));
    let unread = read_receipt(
        &fs,
        Path::new("/nowhere/ALL_VERIFIED.txt"),
        ReceiptKind::AllVerified,
        &Archives::default(),
    );
    assert_eq!(
        (unread.verdict, unread.why.as_deref()),
        ("unreadable", Some("absent"))
    );
    let mut archives = Archives::with_capacity(1);
    archives.add(
        PathBuf::from("/arch/2020.zip"),
        meta(Kind::File, 1, 9, 1, 4),
    );
    let mut identity = SourceIdentity {
        dev: 77,
        ino: 9,
        size: 4,
        mtime_s: 1_700_000_000,
        mtime_ns: 7,
        ctime_s: 1_700_000_001,
        ctime_ns: 9,
    };
    assert_eq!(archives.find(&identity), Some(Path::new("/arch/2020.zip")));
    identity.mtime_ns = 8;
    assert_eq!(
        archives.find(&identity),
        None,
        "same inode and size, other times"
    );
}

/// A `source_identity` TORN AT ANY FIELD proves nothing: each of its seven
/// fields missing, or not a number, or an eighth field, is refused; the
/// extremes each type holds are read exactly (a time before 1970, the
/// largest device and inode). And a folder the host cannot list is the
/// host's refusal, never an empty folder.
#[test]
fn a_torn_identity_is_refused_at_every_field_and_an_unlistable_folder_is_the_hosts_refusal() {
    let whole = [77_i64, 9, 4, 1_700_000_000, 7, 1_700_000_001, 9].map(|field| field.to_string());
    assert_eq!(
        SourceIdentity::parse(&format!(" {}\n", whole.join(":"))),
        Some(SourceIdentity {
            dev: 77,
            ino: 9,
            size: 4,
            mtime_s: 1_700_000_000,
            mtime_ns: 7,
            ctime_s: 1_700_000_001,
            ctime_ns: 9,
        })
    );
    for kept in 0..whole.len() {
        let torn = whole[..kept].join(":");
        assert_eq!(
            SourceIdentity::parse(&torn),
            None,
            "{kept} field(s): {torn:?}"
        );
    }
    for bad in 0..whole.len() {
        let mut fields = whole.clone();
        fields[bad] = "not a number".to_owned();
        let spelt = fields.join(":");
        assert_eq!(
            SourceIdentity::parse(&spelt),
            None,
            "field {bad}: {spelt:?}"
        );
    }
    let eight = format!("{}:1", whole.join(":"));
    assert_eq!(SourceIdentity::parse(&eight), None, "an eighth field");
    let edges = format!(
        "{}:{}:0:-1:{}:{}:-1",
        u64::MAX,
        u64::MAX,
        i64::MIN,
        i64::MAX
    );
    assert_eq!(
        SourceIdentity::parse(&edges),
        Some(SourceIdentity {
            dev: u64::MAX,
            ino: u64::MAX,
            size: 0,
            mtime_s: -1,
            mtime_ns: i64::MIN,
            ctime_s: i64::MAX,
            ctime_ns: -1,
        })
    );
    assert_eq!(
        SourceIdentity::parse("-1:9:4:1:2:3:4"),
        None,
        "a negative device"
    );

    let base = scratch("went-away");
    put(&base.join("abcd"), b"abcd");
    let mut names = Vec::new();
    Host.list(&base, &mut |name| names.push(name)).unwrap();
    assert_eq!(names, vec![OsString::from("abcd")]);
    let mut met = 0;
    let gone = Host.list(&base.join("went-away"), &mut |_| met += 1);
    assert_eq!(
        gone.map_err(|error| error.kind()),
        Err(io::ErrorKind::NotFound)
    );
    assert!(Host.list(&base.join("abcd"), &mut |_| met += 1).is_err());
    assert_eq!(met, 0, "a refused listing names nothing");
    let _ = std::fs::remove_dir_all(&base);
}

/// WHAT TO TAKE CARE OF is said on the retirement list: a folder inside a
/// live root its reader does not read names that root and says to retire
/// only the folder; a folder holding a second name of a live file says its
/// bytes are not counted; and a register with nothing to retire says so in
/// words, never as a blank list.
#[test]
fn the_retirement_list_says_what_to_take_care_of_and_when_nothing_is_left() {
    let fs = Fake::default();
    fs.dir("/ticks", 1, 1);
    fs.dir("/ticks/prove", 1, 2);
    fs.file("/ticks/prove/p.out", 1, 3, 1, b"proof");
    fs.dir("/arch", 1, 4);
    fs.file("/arch/2020.zip", 1, 5, 2, b"zipzip");
    fs.dir("/queue", 1, 6);
    fs.file("/queue/2020.preserved.1.zip.part", 1, 5, 2, b"zipzip");
    let register = walk(
        &fs,
        &config_of(
            &[Path::new("/ticks/prove"), Path::new("/queue")],
            vec![
                live(LiveKind::TickStore, Path::new("/ticks")),
                live(LiveKind::NfoArchives, Path::new("/arch")),
            ],
        ),
        0,
    );
    let list = retirement(&register);
    let caution = |path: &str| {
        list.iter()
            .find(|retire| retire.path == Path::new(path) && !retire.hazard)
            .and_then(|retire| retire.caution.clone())
            .unwrap()
    };
    assert!(
        caution("/ticks/prove").contains("lies inside the live tick_store root \"/ticks\""),
        "{list:?}"
    );
    assert!(
        caution("/queue").starts_with("1 name(s) in it are second names of live files"),
        "{list:?}"
    );
    let empty = walk(&fs, &config_of(&[], Vec::new()), 0);
    assert!(
        retirement_text(&empty).contains("Nothing to retire: every copy root walked has a reader")
    );
}

/// A CLOCK BEFORE 1970 is a negative count of microseconds, never a
/// refusal or a zero; one after it is positive.
#[test]
fn a_wall_clock_before_1970_is_negative_micros() {
    let epoch = std::time::UNIX_EPOCH;
    assert_eq!(
        micros_of(epoch - std::time::Duration::from_secs(1)),
        -1_000_000
    );
    assert_eq!(
        micros_of(epoch + std::time::Duration::from_millis(1_500)),
        1_500_000
    );
    assert!(now_micros() > 0);
}

/// THE ANSWER SAYS WHY IT WALKED OR DID NOT: other configured roots walk
/// again and say so; a request that meets another holding the walk's lock
/// is never made to wait, and is served the last walk marked `rewalking`.
#[test]
fn a_changed_configuration_walks_and_a_held_lock_serves_the_last_walk() {
    let fs = Fake::default();
    fs.dir("/one", 1, 1);
    fs.dir("/two", 1, 2);
    let cache = Cache::new();
    let first = config_of(&[Path::new("/one")], Vec::new());
    let second = config_of(&[Path::new("/one"), Path::new("/two")], Vec::new());
    assert!(matches!(
        answer(&cache, &fs, &first, &|| 1),
        Served::Register { fresh: true, .. }
    ));
    let Served::Register { why, fresh, .. } = answer(&cache, &fs, &second, &|| 2) else {
        panic!("a walk");
    };
    assert!(fresh);
    assert_eq!(why, "the configured roots changed since the last walk");
    let walking = cache.walking.lock().unwrap();
    let Served::Register {
        register,
        why,
        fresh,
        rewalking,
        ..
    } = answer(&cache, &fs, &first, &|| 3)
    else {
        panic!("the held walk");
    };
    assert!(rewalking && !fresh, "{why}");
    assert!(
        why.starts_with("being re-walked by another request (the configured roots changed"),
        "{why}"
    );
    assert_eq!(
        register.roots.len(),
        2,
        "the last walk, of the second roots"
    );
    assert!(
        matches!(
            answer(&cache, &fs, &second, &|| 4),
            Served::Register {
                fresh: false,
                rewalking: false,
                ..
            }
        ),
        "an unchanged answer is served whoever holds the lock"
    );
    drop(walking);
}

/// THE LAST EDGES: an identity differing in any one compared field is not a
/// match (`st_dev` never compared); an empty environment fallback is unset,
/// never a folder; a live root that is not walked does not make the
/// register partial; a retirement line with nothing to take care of says
/// nothing more; and the process has exactly one cache.
#[test]
fn every_compared_field_unset_fallback_and_plain_line_is_exact() {
    let file = meta(Kind::File, 1, 9, 1, 4);
    let same = SourceIdentity {
        dev: 2,
        ino: 9,
        size: 4,
        mtime_s: 1_700_000_000,
        mtime_ns: 7,
        ctime_s: 1_700_000_001,
        ctime_ns: 9,
    };
    assert!(same.matches(&file), "st_dev is never compared");
    for other in [
        SourceIdentity { ino: 8, ..same },
        SourceIdentity { size: 5, ..same },
        SourceIdentity { mtime_s: 1, ..same },
        SourceIdentity {
            mtime_ns: 1,
            ..same
        },
        SourceIdentity { ctime_s: 1, ..same },
        SourceIdentity {
            ctime_ns: 1,
            ..same
        },
    ] {
        assert!(!other.matches(&file), "{other:?}");
    }
    let env = |name: &str| (name == "BRUTEX_MASTERS").then(OsString::new);
    let config = config(&Sources {
        copy_roots: Some("/i/a"),
        source_gdfl: None,
        source_masters: None,
        store_root: Path::new("/i/store"),
        env: &env,
    })
    .unwrap();
    assert!(
        config.unset_live.iter().any(|why| why
            == "source_masters: source.masters is not set, so nothing is compared with it"),
        "{:?}",
        config.unset_live
    );
    let fs = Fake::default();
    fs.dir("/plain", 1, 1);
    fs.file("/plain/a.csv", 1, 2, 1, b"abc");
    fs.dir("/store", 1, 3);
    let register = walk(
        &fs,
        &config_of(
            &[Path::new("/plain")],
            vec![live(LiveKind::Store, Path::new("/store"))],
        ),
        0,
    );
    assert!(!register.live[0].walked);
    assert!(register.whole(), "a store not walked is not a partial walk");
    let text = retirement_text(&register);
    assert!(
        text.contains("1. \"/plain\"\n   bytes freed: 3 in 1 file(s)"),
        "{text}"
    );
    assert!(!text.contains("take care"), "{text}");
    assert!(std::ptr::eq(process_cache(), process_cache()));
}

/// A ROOT WHOSE OWN STAT CHANGED is named in the answer's `why`.
#[test]
fn a_changed_root_is_named_in_the_answer() {
    let fs = Fake::default();
    fs.dir("/one", 1, 1);
    fs.dir("/store", 1, 2);
    let cache = Cache::new();
    let config = config_of(
        &[Path::new("/one")],
        vec![live(LiveKind::Store, Path::new("/store"))],
    );
    let _ = answer(&cache, &fs, &config, &|| 1);
    fs.nodes
        .lock()
        .unwrap()
        .get_mut(Path::new("/one"))
        .unwrap()
        .meta
        .mtime_ns = 8;
    let Served::Register { why, fresh, .. } = answer(&cache, &fs, &config, &|| 2) else {
        panic!("a walk");
    };
    assert!(fresh);
    assert_eq!(why, "the own stat of \"/one\" changed");
}
