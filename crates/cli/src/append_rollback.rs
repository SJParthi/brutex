//! The one append-with-rollback every fixed-stride `cli` ledger appends
//! through (D-1622, D-1850).
//!
//! An append-only ledger refuses a ragged or torn file when it opens. A plain
//! `seek(End) + write_all` that fails part-way (ENOSPC, EIO, a short write)
//! therefore leaves a partial record that wedges the ledger for good. Every
//! append here records the end offset first and, when the write fails,
//! truncates the file back to it, so the bytes on disk are exactly the bytes
//! before the attempt and the next append starts at a record boundary. When
//! the truncation also fails both errors are named: nothing is hidden (§4).
//!
//! A failed durability barrier after a whole write is not this helper's case:
//! the whole record is a valid orphan that each ledger's exact-retry rule
//! already continues, so it is left in place.
//!
//! What it does per append: one `seek` and one write, and one `set_len` only
//! when the write fails.

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

/// Appends `raw` at the end of `file`, truncating back on a failed write.
///
/// # Errors
///
/// Names `label` and the seek, write or truncation error.
pub(crate) fn append(file: &mut File, raw: &[u8], label: &str) -> Result<(), String> {
    append_with(file, raw, label, Write::write_all)
}

/// [`append`] with the write itself supplied, so a test can inject a short
/// write into a real ledger file.
///
/// # Errors
///
/// Names `label` and the seek, write or truncation error.
pub(crate) fn append_with(
    file: &mut File,
    raw: &[u8],
    label: &str,
    write: impl FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
) -> Result<(), String> {
    let end = file
        .seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek {label} append: {why}"))?;
    let Err(why) = write(file, raw) else {
        return Ok(());
    };
    match file.set_len(end) {
        Ok(()) => Err(format!(
            "cannot append {label}: {why}; truncated back to {end} bytes"
        )),
        Err(rollback) => Err(format!(
            "cannot append {label}: {why}; truncation back to {end} bytes also failed: {rollback}"
        )),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "a failed fixture must fail its test")]
pub(crate) mod tests {
    use super::{append, append_with};
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;

    /// Test support: opens the ledger file at `path`, injects a write that lands
    /// part of `width` bytes and then fails, and requires the refusal to name
    /// both the write error and the rollback and the file to be byte-identical
    /// afterwards. Every ledger that appends through [`append`] proves its own
    /// recovery with this before it appends again.
    pub(crate) fn inject_short_write(path: &std::path::Path, label: &str, width: usize) {
        let before = std::fs::read(path).expect("read ledger file before the injected write");
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open ledger file for the injected write");
        let raw = vec![0x5a_u8; width];
        let refusal = append_with(&mut file, &raw, label, |file, raw| {
            file.write_all(raw.get(..raw.len() / 2 + 1).expect("partial record"))?;
            Err(std::io::Error::other("injected short write"))
        })
        .expect_err("a failed write must refuse");
        assert!(
            refusal.contains(label)
                && refusal.contains("injected short write")
                && refusal.contains(&format!("truncated back to {} bytes", before.len())),
            "refusal `{refusal}` must name the ledger, the write error and the rollback"
        );
        drop(file);
        assert_eq!(
            std::fs::read(path).expect("read ledger file after the injected write"),
            before,
            "{} must be byte-identical after a rolled-back append",
            path.display()
        );
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "brutex-append-rollback-{name}-{}",
                std::process::id()
            ));
            drop(std::fs::remove_dir_all(&dir));
            std::fs::create_dir_all(&dir).expect("create scratch directory");
            Self(dir)
        }

        fn file(&self, bytes: &[u8]) -> PathBuf {
            let path = self.0.join("ledger.bin");
            std::fs::write(&path, bytes).expect("seed ledger file");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            drop(std::fs::remove_dir_all(&self.0));
        }
    }

    fn open_rw(path: &std::path::Path) -> std::fs::File {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open read-write")
    }

    #[test]
    fn a_short_write_truncates_back_and_the_next_append_lands_on_the_boundary() {
        let scratch = Scratch::new("short");
        let path = scratch.file(&[1, 2, 3, 4]);
        let mut file = open_rw(&path);
        let refusal = append_with(&mut file, &[9; 8], "Test V1 record", |file, raw| {
            file.write_all(raw.get(..5).expect("partial"))?;
            Err(std::io::Error::other("injected short write"))
        })
        .expect_err("a failed write refuses");
        assert_eq!(
            refusal,
            "cannot append Test V1 record: injected short write; truncated back to 4 bytes"
        );
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2, 3, 4]);
        // The cursor was left past the truncated end; the next append must
        // still land exactly on the old end, with no hole and no overwrite.
        append(&mut file, &[7, 7], "Test V1 record").expect("next append succeeds");
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2, 3, 4, 7, 7]);
    }

    #[test]
    fn a_failed_write_of_nothing_written_still_leaves_the_file_exact() {
        let scratch = Scratch::new("nothing");
        let path = scratch.file(&[]);
        let mut file = open_rw(&path);
        let refusal = append_with(&mut file, &[9; 3], "Empty V1 record", |_, _| {
            Err(std::io::Error::other("ENOSPC"))
        })
        .expect_err("refuses");
        assert!(refusal.ends_with("truncated back to 0 bytes"), "{refusal}");
        assert!(std::fs::read(&path).expect("read").is_empty());
    }

    #[test]
    fn an_append_writes_at_the_end_even_when_the_cursor_is_elsewhere() {
        let scratch = Scratch::new("cursor");
        let path = scratch.file(&[1, 2, 3]);
        let mut file = open_rw(&path);
        file.seek(SeekFrom::Start(0)).expect("rewind");
        append(&mut file, &[4, 5], "Cursor V1 record").expect("append");
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2, 3, 4, 5]);
        append(&mut file, &[], "Cursor V1 record").expect("an empty append is a no-op");
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_failed_truncation_names_both_errors() {
        let scratch = Scratch::new("rollback");
        let path = scratch.file(&[1, 2]);
        // A read-only handle cannot be truncated, so the rollback itself fails.
        let mut file = std::fs::File::open(&path).expect("open read-only");
        let refusal = append_with(&mut file, &[3], "Readonly V1 record", |_, _| {
            Err(std::io::Error::other("injected EIO"))
        })
        .expect_err("refuses");
        assert!(
            refusal.starts_with(
                "cannot append Readonly V1 record: injected EIO; truncation back to 2 bytes also failed: "
            ),
            "{refusal}"
        );
        // And a plain append on that handle names the write error itself.
        let refusal = append(&mut file, &[3], "Readonly V1 record").expect_err("refuses");
        assert!(
            refusal.starts_with("cannot append Readonly V1 record: ")
                && refusal.contains("also failed"),
            "{refusal}"
        );
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2]);
    }

    #[test]
    fn the_test_injector_proves_byte_identity_on_a_real_file() {
        let scratch = Scratch::new("injector");
        let path = scratch.file(&[8; 12]);
        inject_short_write(&path, "Injector V1 record", 6);
        assert_eq!(std::fs::read(&path).expect("read"), [8; 12]);
    }
}
