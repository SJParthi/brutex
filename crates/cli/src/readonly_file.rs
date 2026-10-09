//! Read-only immutable-evidence opens that cannot follow a final symlink or
//! wait for a FIFO peer, and the ledger and directory doors that cannot wait
//! either. Every `cli` and `api` open goes through one of them or carries
//! `O_NONBLOCK` itself; `no_cli_or_api_open_can_wait_for_a_fifo_peer` walks
//! both crates and refuses any other (G5-2, D-4732).
//! Intermediate directories and device/filesystem I/O still require a trusted
//! store root; this is not an `openat` path sandbox.

use std::fs::{File, OpenOptions};
use std::path::Path;

// Per-architecture values live in store::open_flags: O_NOFOLLOW is bit 17 on
// x86_64 Linux but 0x8000 on aarch64, where bit 17 is O_LARGEFILE (D-0980).
#[cfg(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
// One literal per target, not an `|` of the two: over disjoint bits `|` and `^`
// agree, so that expression carried an equivalent mutant (D-0192).
const FLAGS: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;

#[cfg(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
pub(crate) fn open(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FLAGS)
        .open(path)?;
    if !file.metadata()?.file_type().is_file() {
        return Err(std::io::Error::other(
            "immutable evidence path is not a regular file",
        ));
    }
    crate::result_set::file_generation(&file, path).map_err(std::io::Error::other)?;
    Ok(file)
}

/// The cli ledger door (D-1743): open `path` with the caller's `options` plus
/// `O_NONBLOCK`, so a FIFO or socket at a ledger path can never wait for a
/// peer, then refuse any handle that is not a regular file. The type is asked
/// of the HANDLE, so nothing can be swapped between a check and the open.
///
/// Final symlinks are followed, exactly as `store::file::open_read` follows
/// them: a symlink to a regular ledger stays readable and what it reaches is
/// checked. An absent path keeps its `NotFound` kind. `O_NONBLOCK` stays set
/// on the handle and has no effect on a regular file's reads, writes or locks.
///
/// On any other target it refuses with `Unsupported`. ONE function with two
/// cfg'd tails, not two functions: a compiled-out body is still a body
/// cargo-mutants mutates, and no build of this target can compile or kill the
/// mutant (G18-cli-b-21, D-2028).
///
/// Public since G5-2 (D-4732), so `api` opens its own journals and the ledger
/// files it reads through this one door rather than a second copy of it.
///
/// # Errors
///
/// The host's open error, or a refusal naming `path` when the opened handle
/// is not a regular file.
pub fn regular(options: &mut OpenOptions, path: &Path) -> std::io::Result<File> {
    #[cfg(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        let file = options
            .custom_flags(store::open_flags::O_NONBLOCK)
            .open(path)?;
        if file.metadata()?.file_type().is_file() {
            Ok(file)
        } else {
            Err(std::io::Error::other(format!(
                "{} is not a regular file; a ledger is never read from or written to anything else",
                path.display()
            )))
        }
    }
    #[cfg(not(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = (options, path);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "ledger opens require verified macOS or Linux x86_64/aarch64 flags",
        ))
    }
}

/// [`regular`] opened read-only: the drop-in for `File::open` on any ledger,
/// result, journal or lock file `cli` or `api` reads (G5-2, D-4732). It keeps
/// `File::open`'s error kinds, so a caller's `NotFound` arm is unchanged, and
/// it can never wait for a FIFO or socket peer.
///
/// # Errors
///
/// The host's open error, or a refusal naming `path` when the opened handle is
/// not a regular file.
pub fn read(path: impl AsRef<Path>) -> std::io::Result<File> {
    regular(OpenOptions::new().read(true), path.as_ref())
}

/// A directory handle that cannot wait for a FIFO or socket peer: the target of
/// a durability barrier's `fsync`, or a root a ledger holds open (G5-2,
/// D-4732). `File::open` on a directory path set no `O_NONBLOCK`, so a FIFO
/// planted there held the opener forever; this opens read-only with
/// `O_NONBLOCK` and refuses any handle whose `fstat` is not a directory,
/// naming the path. A final symlink is followed, as `File::open` followed it,
/// and an absent path keeps its `NotFound` kind.
///
/// On any other target it refuses with `Unsupported`, for the reason
/// [`regular`] gives.
///
/// # Errors
///
/// The host's open error, or `NotADirectory` naming `path`.
pub fn directory(path: impl AsRef<Path>) -> std::io::Result<File> {
    let path = path.as_ref();
    #[cfg(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(store::open_flags::O_NONBLOCK)
            .open(path)?;
        if file.metadata()?.is_dir() {
            Ok(file)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                format!(
                    "{} is not a directory; a durability barrier or a held root is never anything else",
                    path.display()
                ),
            ))
        }
    }
    #[cfg(not(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "directory opens require verified macOS or Linux x86_64/aarch64 flags",
        ))
    }
}

#[cfg(not(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
pub(crate) fn open(_path: &Path) -> std::io::Result<File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "immutable read opens require verified macOS or Linux x86_64/aarch64 flags",
    ))
}

#[cfg(test)]
#[cfg(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn regular_evidence_opens_but_final_symlinks_and_directories_refuse()
    -> Result<(), Box<dyn std::error::Error>> {
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let path = scratch.0.join("receipt");
        std::fs::write(&path, b"exact bytes")?;
        assert_eq!(open(&path)?.metadata()?.len(), 11);
        let alias = scratch.0.join("alias");
        std::os::unix::fs::symlink(&path, &alias)?;
        assert!(open(&alias).is_err());
        assert!(open(&scratch.0).is_err());
        std::fs::remove_file(&path)?;
        assert!(
            open(&alias).is_err(),
            "a dangling symlink cannot create a target"
        );
        assert!(!path.exists());
        Ok(())
    }

    #[test]
    fn fifo_without_a_writer_refuses_and_the_probe_cannot_hang()
    -> Result<(), Box<dyn std::error::Error>> {
        const CHILD: &str = "BRUTEX_READONLY_FIFO_PROBE";
        if let Some(path) = std::env::var_os(CHILD) {
            let refusal = open(Path::new(&path)).err().ok_or("FIFO was admitted")?;
            assert!(refusal.to_string().contains("not a regular file"));
            return Ok(());
        }
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let path = scratch.0.join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()?
                .success(),
            "the supported Unix test host must create its FIFO fixture"
        );
        // THE CHILD MUST PROVE IT RAN (P1-11-02): `--exact` on a name that
        // matches nothing exits zero, so the path comes from `module_path!()`
        // and the child's own `1 passed` line is required.
        let test = concat!(
            module_path!(),
            "::fifo_without_a_writer_refuses_and_the_probe_cannot_hang"
        );
        let test = test.split_once("::").map_or(test, |(_, path)| path);
        let mut child = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", test])
            .env(CHILD, &path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                let mut stdout = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    std::io::Read::read_to_string(&mut pipe, &mut stdout)?;
                }
                assert!(
                    status.success(),
                    "nonblocking FIFO child must refuse normally: {stdout}"
                );
                assert!(
                    stdout.contains("test result: ok. 1 passed;"),
                    "the child must run exactly this one test:\n{stdout}"
                );
                break;
            }
            if started.elapsed() >= std::time::Duration::from_secs(2) {
                child.kill()?;
                child.wait()?;
                return Err("read-only FIFO open waited for a writer".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }

    /// Make a FIFO with the host's own tool (no `libc` binding).
    fn mkfifo(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let made = std::process::Command::new("mkfifo").arg(path).status()?;
        if made.success() {
            Ok(())
        } else {
            Err(format!("mkfifo {} failed", path.display()).into())
        }
    }

    /// Run `open` on another thread and return its refusal, or fail if it is
    /// still waiting after two seconds. A blocked reader is offered the FIFO's
    /// write end once, so it can usually finish rather than leak.
    fn refuses_promptly(
        fifo: &Path,
        open: impl FnOnce() -> Option<String> + Send + 'static,
    ) -> Result<String, Box<dyn std::error::Error>> {
        use std::os::unix::fs::OpenOptionsExt as _;
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _ = tx.send(open());
        });
        if let Ok(refusal) = rx.recv_timeout(std::time::Duration::from_secs(2)) {
            worker.join().map_err(|_| "the opener panicked")?;
            return refusal.ok_or_else(|| format!("{} was admitted", fifo.display()).into());
        }
        let peer = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(store::open_flags::O_NONBLOCK)
            .open(fifo);
        // Not joined: an old door may block again on a later open, and the
        // refusal below must not wait for it.
        drop((peer, worker));
        Err(format!("opening {} waited for a FIFO peer", fifo.display()).into())
    }

    /// W2-cli13-4: a FIFO planted at a cli ledger path made every read-only
    /// open of it block forever -- including the HTTP detail path through
    /// `CommittedParents::open_read_bounded` -- because those opens set no
    /// `O_NONBLOCK` and asked the handle its type only after `open(2)`
    /// returned. Each door now refuses at once and names the cause. D-1743.
    #[test]
    fn a_fifo_at_any_cli_ledger_path_refuses_without_waiting()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::pre_admission_data::{
            PreAdmissionDataBoundsV1, PreAdmissionDataBoundsV2, PreAdmissionDataLedgerV1,
            PreAdmissionDataLedgerV2,
        };
        let not_regular = |why: &str| why.contains("not a regular file");

        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let root = scratch.0.clone();
        let runs = crate::results::Results::path(&root);
        mkfifo(&runs)?;
        for bounded in [false, true] {
            let at = root.clone();
            let why = refuses_promptly(&runs, move || {
                if bounded {
                    crate::results::Results::open_read_bounded(&at, 1 << 20).err()
                } else {
                    crate::results::Results::open_read(&at).err()
                }
            })?;
            assert!(not_regular(&why), "{why}");
        }
        let at = root.clone();
        let why = refuses_promptly(&runs, move || {
            crate::result_set::CommittedParents::open_read_bounded(&at, 1 << 20).err()
        })?;
        assert!(not_regular(&why), "the HTTP parent reader: {why}");
        std::fs::remove_file(&runs)?;

        let receipts = crate::result_set::Receipts::path(&root);
        mkfifo(&receipts)?;
        for door in 0..3 {
            let at = root.clone();
            let why = refuses_promptly(&receipts, move || match door {
                0 => crate::result_set::Receipts::open_read(&at).err(),
                1 => crate::result_set::Receipts::open_read_bounded(&at, 1 << 20).err(),
                _ => crate::result_set::Receipts::open(&at).err(),
            })?;
            assert!(not_regular(&why), "receipt door {door}: {why}");
        }

        for (lock, data) in [
            ("pre-admission-data-v1.lock", "pre-admission-data-v1.bin"),
            ("pre-admission-data-v2.lock", "pre-admission-data-v2.bin"),
        ] {
            for fifo_at in [lock, data] {
                let ledger = crate::search_checkpoint::tests::Scratch::new()?;
                std::fs::write(ledger.0.join(lock), b"")?;
                std::fs::write(ledger.0.join(data), b"")?;
                std::fs::remove_file(ledger.0.join(fifo_at))?;
                let fifo = ledger.0.join(fifo_at);
                mkfifo(&fifo)?;
                let at = ledger.0.clone();
                let v1 = lock.contains("v1");
                let why = refuses_promptly(&fifo, move || {
                    if v1 {
                        PreAdmissionDataBoundsV1::new(4, 1 << 20)
                            .and_then(|bounds| PreAdmissionDataLedgerV1::open_read(&at, bounds))
                            .err()
                    } else {
                        PreAdmissionDataBoundsV2::new(4, 1 << 20)
                            .and_then(|bounds| PreAdmissionDataLedgerV2::open_read(&at, bounds))
                            .err()
                    }
                })?;
                assert!(not_regular(&why), "{fifo_at}: {why}");
            }
        }

        // G5-2 (D-4732): the HTTP readers D-1743's heading claimed and its
        // body never reached -- `/top.json` and `/frontier.json` through
        // `Frontier`, `/trades.json` through `Trades`.
        let frontier = crate::frontier::Frontier::path(&root);
        mkfifo(&frontier)?;
        for bounded in [false, true] {
            let at = root.clone();
            let why = refuses_promptly(&frontier, move || {
                if bounded {
                    crate::frontier::Frontier::open_read_bounded(&at, 1 << 20).err()
                } else {
                    crate::frontier::Frontier::open_read(&at).err()
                }
            })?;
            assert!(not_regular(&why), "the frontier reader: {why}");
        }
        std::fs::remove_file(&frontier)?;
        let trades = crate::trades::Trades::path(&root);
        mkfifo(&trades)?;
        for bounded in [false, true] {
            let at = root.clone();
            let why = refuses_promptly(&trades, move || {
                if bounded {
                    crate::trades::Trades::open_read_bounded(&at, 1 << 20).err()
                } else {
                    crate::trades::Trades::open_read(&at).err()
                }
            })?;
            assert!(not_regular(&why), "the trades reader: {why}");
        }
        std::fs::remove_file(&trades)?;

        // `/sweep-evidence.json` and `/candidate.json`: every sweep-evidence
        // file a reader opens -- the global journal, the identity's start
        // index, one attempt's lifecycle and reservation, and a child detail
        // file both counted and paged.
        sweep_evidence_fifos_refuse(&not_regular)?;
        Ok(())
    }

    /// Each sweep-evidence reader door, a FIFO planted in place of the file it
    /// opens. The attempt is finished before any FIFO exists, so no writer of
    /// this test can reach one.
    fn sweep_evidence_fifos_refuse(
        not_regular: &dyn Fn(&str) -> bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::sweep_evidence::{self as evidence, Completion, Operation};
        const BOUND: u64 = 1 << 20;
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let root = scratch.0.clone();
        let identity = [0x5a_u8; 32];
        let attempt = evidence::begin(&root, identity, Operation::Sweep)?;
        let token = attempt.token();
        attempt.finish(Completion::Completed)?;
        let read = evidence::read(&root, identity, BOUND)?.ok_or("the attempt reads back")?;
        let base = root.join("results").join("sweep-evidence-v1");
        let own = base.join("5a".repeat(32));

        // A child detail file the evidence declares empty: counted by every
        // read, opened by every page.
        let levels = own.join(format!("{token}-levels.bin"));
        mkfifo(&levels)?;
        let at = root.clone();
        let why = refuses_promptly(&levels, move || evidence::read(&at, identity, BOUND).err())?;
        assert!(not_regular(&why), "the counted child: {why}");
        let at = root.clone();
        let why = refuses_promptly(&levels, move || {
            evidence::depth_page(&at, &read, 0, 1, BOUND).err()
        })?;
        assert!(not_regular(&why), "the paged child: {why}");
        std::fs::remove_file(&levels)?;

        for (file, door) in [
            (base.join("attempts.bin"), 0),
            (own.join("starts.bin"), 1),
            (own.join(format!("{token}-lifecycle.bin")), 2),
            (base.join(format!("{token}-start.bin")), 1),
        ] {
            let aside = file.with_extension("aside");
            std::fs::rename(&file, &aside)?;
            mkfifo(&file)?;
            let at = root.clone();
            let why = refuses_promptly(&file, move || match door {
                0 => evidence::latest(&at, BOUND).err(),
                1 => evidence::read(&at, identity, BOUND).err(),
                _ => evidence::read_attempt(&at, identity, token, BOUND).err(),
            })?;
            assert!(not_regular(&why), "{}: {why}", file.display());
            std::fs::remove_file(&file)?;
            std::fs::rename(&aside, &file)?;
        }
        assert!(
            evidence::read(&root, identity, BOUND)?.is_some(),
            "every displaced file was restored and reads again"
        );
        Ok(())
    }
    /// The ledger door: a socket and a directory refuse by name, a symlink to
    /// a regular file is followed exactly as the store's own read door follows
    /// it, and an absent file keeps its `NotFound` kind for the callers that
    /// report absence as no run recorded.
    #[test]
    fn the_ledger_door_refuses_sockets_and_directories_and_keeps_not_found()
    -> Result<(), Box<dyn std::error::Error>> {
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let reading = || {
            let mut options = std::fs::OpenOptions::new();
            options.read(true);
            options
        };
        let socket = scratch.0.join("socket");
        let _listener = std::os::unix::net::UnixListener::bind(&socket)?;
        assert!(regular(&mut reading(), &socket).is_err());
        let refusal = regular(&mut reading(), &scratch.0)
            .err()
            .ok_or("a directory opened")?;
        assert!(
            refusal.to_string().contains("not a regular file"),
            "{refusal}"
        );
        let missing = regular(&mut reading(), &scratch.0.join("absent"))
            .err()
            .ok_or("an absent file opened")?;
        assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);
        let target = scratch.0.join("ledger");
        std::fs::write(&target, b"bytes")?;
        let alias = scratch.0.join("alias");
        std::os::unix::fs::symlink(&target, &alias)?;
        assert_eq!(regular(&mut reading(), &alias)?.metadata()?.len(), 5);
        let mut writing = std::fs::OpenOptions::new();
        writing.read(true).write(true).create(true).truncate(false);
        let fresh = scratch.0.join("fresh");
        assert_eq!(regular(&mut writing, &fresh)?.metadata()?.len(), 0);
        let fifo = scratch.0.join("fifo");
        mkfifo(&fifo)?;
        let refusal = regular(&mut writing, &fifo)
            .err()
            .ok_or("a FIFO opened for writing")?;
        assert!(
            refusal.to_string().contains("not a regular file"),
            "{refusal}"
        );
        Ok(())
    }

    /// The premise the scan below takes for a read-write `OpenOptions`: with
    /// no `O_NONBLOCK`, `open(2)` admits a FIFO for `O_RDWR` at once (fifo(7)),
    /// where `O_RDONLY` waits for a writer. So a read-write open needs no door
    /// to be bounded, and the serve lock's keeps reaching a device that refuses
    /// its stamp (G5-2, D-4732).
    #[test]
    fn a_read_write_open_of_a_fifo_never_waits() -> Result<(), Box<dyn std::error::Error>> {
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let fifo = scratch.0.join("fifo");
        mkfifo(&fifo)?;
        let path = fifo.clone();
        let opened = refuses_promptly(&fifo, move || {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .ok()
                .map(|file| format!("opened {}", file.metadata().is_ok()))
        })?;
        assert_eq!(opened, "opened true");
        let path = fifo.clone();
        let waited = refuses_promptly(&fifo, move || {
            std::fs::File::open(&path).ok().map(|_| "opened".to_owned())
        })
        .err()
        .ok_or("premise: a read-only open of a FIFO waits for a writer")?;
        assert!(
            waited.to_string().contains("waited for a FIFO peer"),
            "{waited}"
        );
        Ok(())
    }

    /// `source` with every `#[cfg(test)]` item removed, so a scan reads only
    /// what a release build compiles, each removed line left empty so line numbers
    /// still name the file's own lines, and the file names of the test-only
    /// `mod NAME;` declarations it removed (`#[path]` honoured). The item an
    /// attribute run covers ends at its own `;`, or at the first later line
    /// that closes its brace at the same indentation, which `cargo fmt
    /// --check` makes exact.
    fn split_release(source: &str) -> (String, Vec<(String, bool)>) {
        let lines: Vec<&str> = source.lines().collect();
        let mut kept = String::with_capacity(source.len());
        let mut gated = Vec::new();
        let mut at = 0_usize;
        while let Some(line) = lines.get(at) {
            let trimmed = line.trim_start();
            if !(trimmed.starts_with("#[cfg(test") || trimmed.starts_with("#[cfg(all(test")) {
                kept.push_str(line);
                kept.push('\n');
                at += 1;
                continue;
            }
            // Further attributes and comments between the `cfg` and its item.
            let mut item = at + 1;
            let mut path_attribute = None;
            while let Some(next) = lines.get(item) {
                let next = next.trim_start();
                if next.starts_with("#[") {
                    if let Some(named) = next.strip_prefix("#[path = \"") {
                        path_attribute = named.split('"').next().map(str::to_owned);
                    }
                    while lines
                        .get(item)
                        .is_some_and(|l| !l.trim_end().ends_with(']'))
                    {
                        item += 1;
                    }
                    item += 1;
                } else if next.starts_with("//") {
                    item += 1;
                } else {
                    break;
                }
            }
            let start = at;
            let Some(head) = lines.get(item) else { break };
            at = item + 1;
            let declared = head
                .trim()
                .trim_start_matches("pub(crate) ")
                .trim_start_matches("pub(super) ")
                .trim_start_matches("pub ")
                .strip_prefix("mod ")
                .and_then(|rest| rest.strip_suffix(';'));
            if let Some(name) = declared {
                gated.push(match path_attribute {
                    Some(named) => (named, true),
                    None => (format!("{name}.rs"), false),
                });
            }
            if head.trim_end().ends_with('{') {
                let indent = &head[..head.len() - head.trim_start().len()];
                let close = format!("{indent}}}");
                while let Some(body) = lines.get(at) {
                    at += 1;
                    let body = body.trim_end();
                    if body == close || body == format!("{close};") {
                        break;
                    }
                }
            }
            // One empty line per removed line, so a line number in the release
            // text is the same line in the file.
            for _ in start..at {
                kept.push('\n');
            }
        }
        (kept, gated)
    }

    /// Every `.rs` file under `dir`, recursively, in path order.
    fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Every `.rs` file under `dir` that a release build compiles, with its
    /// `#[cfg(test)]` items removed: not a `*_tests.rs` or `tests.rs` file,
    /// not one opening `#![cfg(test)]`, not one a parent declares only under
    /// `#[cfg(test)]`, and nothing beneath such a file's own module directory.
    pub(crate) fn release_sources(dir: &Path, out: &mut Vec<(std::path::PathBuf, String)>) {
        let mut files = Vec::new();
        rust_files(dir, &mut files);
        let mut test_only: Vec<std::path::PathBuf> = Vec::new();
        let mut kept = Vec::new();
        for path in files {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let (release, gated) = split_release(&text);
            let parent = path.parent().unwrap_or(dir);
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let children = if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
                parent.to_path_buf()
            } else {
                path.with_extension("")
            };
            for (module, attributed) in gated {
                if attributed {
                    // `#[path]` is relative to the declaring file's directory.
                    test_only.push(parent.join(module));
                } else {
                    test_only.push(children.join(&module));
                    test_only.push(children.join(module.trim_end_matches(".rs")).join("mod.rs"));
                }
            }
            let whole_file_test = name.ends_with("_tests.rs")
                || name == "tests.rs"
                || text.lines().take(40).any(|l| l.trim() == "#![cfg(test)]");
            if !whole_file_test {
                kept.push((path, release));
            }
        }
        for (path, release) in kept {
            let gated = test_only.iter().any(|test| {
                *test == path
                    || path.starts_with(test.with_extension(""))
                    || (test.ends_with("mod.rs")
                        && test.parent().is_some_and(|module| path.starts_with(module)))
            });
            if !gated {
                out.push((path, release));
            }
        }
    }

    /// Whether the `OpenOptions` built at `at` cannot wait in `open(2)`: handed to
    /// `regular`, opened read-write (`.read(true)` with `.write(true)` or
    /// `.append(true)`, which `a_read_write_open_of_a_fifo_never_waits` shows a
    /// FIFO admits at once), or given `custom_flags` naming a non-blocking flag
    /// -- directly, or through a `const` of this file whose value names one --
    /// before the first `.open(` after it.
    fn waits_for_nothing(text: &str, at: usize) -> bool {
        let before = text
            .get(..at)
            .unwrap_or("")
            .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == ':')
            .trim_end();
        if before.ends_with("regular(") {
            return true;
        }
        let rest = text.get(at..).unwrap_or("");
        let end = rest
            .find(".open(")
            .map_or(rest.len(), |open| open + ".open(".len());
        let window = rest.get(..end.min(2_000)).unwrap_or(rest);
        if window.contains("readonly_file::regular(") {
            return true;
        }
        if window.contains(".read(true)")
            && (window.contains(".write(true)") || window.contains(".append(true)"))
        {
            return true;
        }
        let Some(flags) = window.find("custom_flags(") else {
            return false;
        };
        if window.contains("NONBLOCK") {
            return true;
        }
        let argument = window
            .get(flags + "custom_flags(".len()..)
            .and_then(|tail| tail.split(')').next())
            .unwrap_or("")
            .trim();
        text.split(&format!("const {argument}: i32 ="))
            .nth(1)
            .and_then(|value| value.split(';').next())
            .is_some_and(|value| value.contains("NONBLOCK"))
    }

    /// G5-2 (D-4732): no read, write or directory open a `cli` or `api`
    /// release build compiles can wait on a FIFO or socket peer. `File::open`
    /// and `File::create` set no `O_NONBLOCK` and are refused outright; every
    /// `OpenOptions` must reach `readonly_file::regular` or carry a
    /// non-blocking custom flag. D-1743 fixed five doors under a heading that
    /// claimed every ledger, and six HTTP readers kept the blocking open; this
    /// walks every source file, so a new door cannot slip past by not being
    /// listed.
    #[test]
    fn no_cli_or_api_open_can_wait_for_a_fifo_peer() {
        let cli = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let api = Path::new(env!("CARGO_MANIFEST_DIR")).join("../api/src");
        let mut sources = Vec::new();
        release_sources(&cli, &mut sources);
        let cli_files = sources.len();
        release_sources(&api, &mut sources);
        assert!(
            cli_files > 100 && sources.len() > cli_files + 40,
            "premise: both crates were walked ({cli_files} cli, {} total)",
            sources.len()
        );
        let lib = sources
            .iter()
            .find(|(path, _)| path.ends_with("cli/src/lib.rs"))
            .map(|(_, text)| text.lines().filter(|line| !line.trim().is_empty()).count());
        assert!(
            lib.is_some_and(|lines| lines > 10_000),
            "premise: lib.rs is read past its first test module ({lib:?} lines)"
        );
        let mut waiting = Vec::new();
        for (path, text) in &sources {
            for pattern in ["File::open(", "File::create("] {
                for (at, _) in text.match_indices(pattern) {
                    let named = text
                        .get(..at)
                        .and_then(|head| head.chars().next_back())
                        .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    if !named {
                        waiting.push(format!("{}: {pattern}", path.display()));
                    }
                }
            }
            for (at, _) in text.match_indices("OpenOptions::new()") {
                if !waits_for_nothing(text, at) {
                    let line = text.get(..at).map_or(0, |head| head.lines().count());
                    waiting.push(format!("{}:{line}: OpenOptions", path.display()));
                }
            }
        }
        assert!(
            waiting.is_empty(),
            "{} open(s) can wait for a FIFO peer:\n{}",
            waiting.len(),
            waiting.join("\n")
        );
    }
}
