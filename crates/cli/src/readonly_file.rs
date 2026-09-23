//! Read-only immutable-evidence opens that cannot follow a final symlink or
//! wait for a FIFO peer. Intermediate directories and device/filesystem I/O
//! still require a trusted store root; this is not an `openat` path sandbox.

use std::fs::File;
use std::path::Path;

#[cfg(target_os = "macos")]
// Local macOS SDK sys/fcntl.h: O_NOFOLLOW=0x100, O_NONBLOCK=0x4.
const FLAGS: i32 = 0x100 | 0x4;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
// Linux UAPI asm-generic/fcntl.h: O_NOFOLLOW=(1<<17), O_NONBLOCK=(1<<11).
// https://github.com/torvalds/linux/blob/master/include/uapi/asm-generic/fcntl.h
const FLAGS: i32 = 0x20_000 | 0x800;

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
}
