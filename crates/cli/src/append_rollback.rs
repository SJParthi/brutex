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
//! A failed durability barrier after a whole write is not this helper's case,
//! and it is NOT safe to leave in place: after a failed `fsync` the pages are
//! clean and readable, so a later barrier "confirms" bytes that never reached
//! the device. A live ledger routes its barriers through
//! `crate::fixed_tail::sync_or_roll_back`, which cuts the block (D-1900,
//! D-2555).
//!
//! What it does per append: one `seek` and one write, and one `set_len` and
//! one `sync_all` only when the write fails (the cut is made durable, D-4460).
//!
//! A process that DIES mid-write (kill, OOM, power loss) runs no rollback and
//! leaves a sub-record tail. That is not this helper's case either: every
//! ledger that appends through it cuts such a tail when its writer opens,
//! under its exclusive lock, with `fixed_tail::heal_torn_tail`, loudly
//! (rnew-1, D-4460). Readers keep refusing it.

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
    append_all(file, label, |file| {
        write(file, raw).map_err(|why| why.to_string())
    })
}

/// [`append`] for an append made of several writes, or of writes with
/// encoding between them (D-1854): `body` writes at the end of `file`, and any
/// error it returns, an I/O error or an encoding refusal after some rows were
/// written, truncates the file back to its length before `body` ran.
///
/// # Errors
///
/// Names `label` and the seek error, `body`'s error, or both `body`'s error
/// and the truncation error.
pub(crate) fn append_all(
    file: &mut File,
    label: &str,
    body: impl FnOnce(&mut File) -> Result<(), String>,
) -> Result<(), String> {
    let end = file
        .seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek {label} append: {why}"))?;
    let Err(why) = body(file) else {
        return Ok(());
    };
    // THE CUT IS MADE DURABLE BEFORE THE REFUSAL RETURNS (rnew-1, D-4460). An
    // unsynced `set_len` can be undone by a power loss, bringing back the
    // partial record this rollback exists to remove.
    match file.set_len(end).and_then(|()| sync_cut(file, label)) {
        Ok(()) => Err(format!(
            "cannot append {label}: {why}; truncated back to {end} bytes"
        )),
        Err(rollback) => Err(format!(
            "cannot append {label}: {why}; truncation back to {end} bytes also failed: {rollback}"
        )),
    }
}

/// The barrier after a rollback's cut, through a test fault hook so a test
/// can prove a failed barrier is named rather than reported as a clean cut.
fn sync_cut(file: &File, label: &str) -> std::io::Result<()> {
    #[cfg(test)]
    if tests::take_cut_fault(label) {
        return Err(std::io::Error::other("injected rollback barrier fault"));
    }
    #[cfg(not(test))]
    let _ = label;
    file.sync_all()
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "a failed fixture must fail its test")]
pub(crate) mod tests {
    use super::{append, append_all, append_with};
    use std::cell::RefCell;
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;

    std::thread_local! {
        /// A label whose next rollback barrier fails, armed by a test.
        static CUT_FAULT: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Whether the rollback barrier for `label` is armed to fail; disarms it.
    pub(super) fn take_cut_fault(label: &str) -> bool {
        CUT_FAULT.with(|armed| {
            let mut armed = armed.borrow_mut();
            if armed.as_deref() == Some(label) {
                *armed = None;
                return true;
            }
            false
        })
    }

    /// rnew-1, D-4460: the rollback's cut is followed by a barrier, and a
    /// barrier that fails is named as a failed rollback, never as a clean cut.
    #[test]
    fn a_rollback_cut_is_synced_and_a_failed_barrier_is_named() {
        let scratch = Scratch::new("cut-barrier");
        let path = scratch.file(&[1, 2, 3]);
        let mut file = open_rw(&path);
        CUT_FAULT.with(|armed| *armed.borrow_mut() = Some("Cut V1 record".to_owned()));
        assert!(!take_cut_fault("Other V1 record"), "another label passes");
        let refusal = append_with(&mut file, &[9; 4], "Cut V1 record", |file, raw| {
            file.write_all(raw.get(..2).expect("partial"))?;
            Err(std::io::Error::other("injected short write"))
        })
        .expect_err("refuses");
        assert_eq!(
            refusal,
            "cannot append Cut V1 record: injected short write; truncation back to 3 bytes also failed: injected rollback barrier fault"
        );
        assert!(!take_cut_fault("Cut V1 record"), "the fault fired once");
        // The cut itself still happened; only its barrier was refused.
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2, 3]);
        let source = include_str!("append_rollback.rs");
        let body = source
            .split_once("pub(crate) fn append_all(")
            .and_then(|(_, rest)| rest.split_once("\n}\n"))
            .expect("append_all body")
            .0;
        assert!(
            body.contains("file.set_len(end).and_then(|()| sync_cut(file, label))"),
            "{body}"
        );
        let cut = source
            .split_once("fn sync_cut(")
            .and_then(|(_, rest)| rest.split_once("\n}\n"))
            .expect("sync_cut body")
            .0;
        assert!(cut.trim_end().ends_with("file.sync_all()"), "{cut}");
    }

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

    /// AHA-05 (D-1854). Several writes are one append: an error after whole
    /// rows, an encoding refusal as much as an I/O error, truncates every
    /// row back, and a body that succeeds keeps all of them.
    #[test]
    fn a_multi_write_append_is_rolled_back_whole_on_any_error() {
        let scratch = Scratch::new("all");
        let path = scratch.file(&[1, 2, 3]);
        let mut file = open_rw(&path);
        let refusal = append_all(&mut file, "Test rows", |file| {
            file.write_all(&[4, 4]).map_err(|why| why.to_string())?;
            file.write_all(&[5, 5]).map_err(|why| why.to_string())?;
            Err("row 3 does not encode".to_owned())
        })
        .expect_err("an encoding refusal refuses");
        assert_eq!(
            refusal,
            "cannot append Test rows: row 3 does not encode; truncated back to 3 bytes"
        );
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2, 3]);
        append_all(&mut file, "Test rows", |file| {
            file.write_all(&[6]).map_err(|why| why.to_string())?;
            file.write_all(&[7]).map_err(|why| why.to_string())
        })
        .expect("a whole body appends");
        assert_eq!(std::fs::read(&path).expect("read"), [1, 2, 3, 6, 7]);
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
