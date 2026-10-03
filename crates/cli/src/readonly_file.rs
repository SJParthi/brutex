//! Read-only immutable-evidence opens that cannot follow a final symlink or
//! wait for a FIFO peer, and the cli ledger door that cannot wait either.
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
const FLAGS: i32 = store::open_flags::O_NOFOLLOW | store::open_flags::O_NONBLOCK;

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

#[cfg(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
/// The cli ledger door (D-1743): open `path` with the caller's `options` plus
/// `O_NONBLOCK`, so a FIFO or socket at a ledger path can never wait for a
/// peer, then refuse any handle that is not a regular file. The type is asked
/// of the HANDLE, so nothing can be swapped between a check and the open.
///
/// Final symlinks are followed, exactly as `store::file::open_read` follows
/// them: a symlink to a regular ledger stays readable and what it reaches is
/// checked. An absent path keeps its `NotFound` kind. `O_NONBLOCK` stays set
/// on the handle and has no effect on a regular file's reads, writes or locks.
pub(crate) fn regular(options: &mut OpenOptions, path: &Path) -> std::io::Result<File> {
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
pub(crate) fn regular(_options: &mut OpenOptions, _path: &Path) -> std::io::Result<File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "ledger opens require verified macOS or Linux x86_64/aarch64 flags",
    ))
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
mod tests {
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
        let mut child = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "readonly_file::tests::fifo_without_a_writer_refuses_and_the_probe_cannot_hang",
            ])
            .env(CHILD, &path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                assert!(
                    status.success(),
                    "nonblocking FIFO child must refuse normally"
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
}
